use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::error::AppError;

/// 登录限流参数(动态可调,系统设置变更后立即生效)。
#[derive(Debug, Clone, Copy)]
pub struct LoginLimits {
    pub max_failures: usize,
    pub lock_window_secs: u64,
}

/// 登录失败节流器：按「(真实 IP, 用户名) 组合键」与「真实 IP 失败总数」双维度计数。
///
/// - 键由调用方传入真实客户端地址（直连 = TCP 对端；经可信反代 = 解析后的 X-Real-IP）。
/// - **组合键维度**（`{ip}|{username}`）：任一 (IP, 用户名) 对在窗口内失败达到阈值即锁定该组合。
///   消除旧版「全局 IP 维度」与「全局用户名维度」锁定放大：攻击者无法用随机凭据
///   锁死全站登录，也无法跨 IP 定向锁死单一账号（DoS 放大防护）。
/// - **单 IP 总数维度**（`ip:{ip}`）：单 IP 累计失败达到阈值即锁定该 IP，
///   防止单 IP 批量枚举不同用户名。
/// - 无全局锁：不会出现「任何人刷 5 次就锁全站」的 DoS 放大。
/// - 限流参数（max_failures / window）通过系统设置动态调整，`update_limits` 立即生效。
pub struct LoginThrottle {
    /// 组合键（IP|用户名）失败时间戳
    pair_map: HashMap<String, Vec<Instant>>,
    /// 单 IP 失败总数时间戳
    ip_map: HashMap<String, Vec<Instant>>,
    limits: std::sync::RwLock<LoginLimits>,
}

impl LoginThrottle {
    pub fn new(max_failures: usize, window_secs: u64) -> Self {
        Self {
            pair_map: HashMap::new(),
            ip_map: HashMap::new(),
            limits: std::sync::RwLock::new(LoginLimits {
                max_failures,
                lock_window_secs: window_secs,
            }),
        }
    }

    fn current_limits(&self) -> LoginLimits {
        *self
            .limits
            .read()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// 动态更新限流参数（系统设置保存后调用，立即生效）。
    /// 参数变更后清空已有失败计数，避免旧阈值下的锁定残留。
    pub fn update_limits(&mut self, max_failures: usize, window_secs: u64) {
        if let Ok(mut l) = self.limits.write() {
            l.max_failures = max_failures;
            l.lock_window_secs = window_secs;
        }
        self.pair_map.clear();
        self.ip_map.clear();
    }

    /// 清理窗口外的时间戳；空条目一并移除，防止 map 无限增长。
    fn prune(&mut self, now: Instant) {
        let window = Duration::from_secs(self.current_limits().lock_window_secs);
        for map in [&mut self.pair_map, &mut self.ip_map] {
            map.retain(|_, times| {
                times.retain(|t| now.duration_since(*t) <= window);
                !times.is_empty()
            });
        }
    }

    fn locked_retry_after(&self, times: &[Instant], now: Instant) -> Option<u64> {
        let limits = self.current_limits();
        let window = Duration::from_secs(limits.lock_window_secs);
        let recent: Vec<&Instant> = times
            .iter()
            .filter(|t| now.duration_since(**t) <= window)
            .collect();
        if recent.len() >= limits.max_failures {
            let oldest = recent.iter().min().copied().unwrap_or(&now);
            let retry_after = window.saturating_sub(now.duration_since(*oldest));
            Some(retry_after.as_secs().max(1))
        } else {
            None
        }
    }

    /// 检查是否锁定；任一维度命中即返回 429（含剩余等待秒数）。
    pub fn check(&mut self, ip: &str, username: &str) -> Result<(), AppError> {
        let now = Instant::now();
        self.prune(now);
        let pair_key = format!("{ip}|{username}");
        let ip_key = format!("ip:{ip}");
        if let Some(secs) = self
            .locked_retry_after(self.pair_map.get(&pair_key).map(Vec::as_slice).unwrap_or(&[]), now)
            .or_else(|| {
                self.locked_retry_after(self.ip_map.get(&ip_key).map(Vec::as_slice).unwrap_or(&[]), now)
            })
        {
            return Err(AppError::TooManyRequests {
                retry_after_secs: secs,
            });
        }
        Ok(())
    }

    /// 记录一次登录失败（组合键与单 IP 总数双维度同时计数）。
    pub fn record_failure(&mut self, ip: &str, username: &str) {
        let now = Instant::now();
        self.prune(now);
        self.pair_map
            .entry(format!("{ip}|{username}"))
            .or_default()
            .push(now);
        self.ip_map.entry(format!("ip:{ip}")).or_default().push(now);
    }

    /// 登录成功时清除该 (IP, 用户名) 与该 IP 的失败记录。
    pub fn clear(&mut self, ip: &str, username: &str) {
        self.pair_map.remove(&format!("{ip}|{username}"));
        self.ip_map.remove(&format!("ip:{ip}"));
    }
}

pub type SharedLoginThrottle = Arc<Mutex<LoginThrottle>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_pair_after_max_failures_from_same_ip() {
        let mut t = LoginThrottle::new(3, 900);
        t.record_failure("10.0.0.1", "alice");
        t.record_failure("10.0.0.1", "alice");
        t.record_failure("10.0.0.1", "alice");
        // 同一 (IP, 用户名) 达到阈值 → 锁定
        assert!(t.check("10.0.0.1", "alice").is_err());
        // 其他 IP 的同一用户名不受影响（消除跨 IP 定向锁死）
        assert!(t.check("10.0.0.2", "alice").is_ok());
    }

    #[test]
    fn locks_ip_total_after_many_usernames() {
        let mut t = LoginThrottle::new(3, 900);
        t.record_failure("10.0.0.1", "a");
        t.record_failure("10.0.0.1", "b");
        t.record_failure("10.0.0.1", "c");
        // 单 IP 总数达到阈值 → 该 IP 被锁（防批量枚举）
        assert!(t.check("10.0.0.1", "zzz_new").is_err());
        // 其他 IP 完全不受影响（无全局锁）
        assert!(t.check("10.0.0.9", "zzz_new").is_ok());
    }

    #[test]
    fn cross_ip_attacker_cannot_lock_victim_account() {
        let mut t = LoginThrottle::new(3, 900);
        // 攻击者从 3 个不同 IP 各失败 2 次
        t.record_failure("1.1.1.1", "victim");
        t.record_failure("1.1.1.1", "victim");
        t.record_failure("2.2.2.2", "victim");
        t.record_failure("2.2.2.2", "victim");
        t.record_failure("3.3.3.3", "victim");
        t.record_failure("3.3.3.3", "victim");
        // 任意 IP 均未达到阈值 → 受害者仍可正常登录
        assert!(t.check("5.5.5.5", "victim").is_ok());
    }

    #[test]
    fn clear_resets_both_dimensions() {
        let mut t = LoginThrottle::new(2, 900);
        t.record_failure("10.0.0.1", "alice");
        t.record_failure("10.0.0.1", "alice");
        assert!(t.check("10.0.0.1", "alice").is_err());
        t.clear("10.0.0.1", "alice");
        assert!(t.check("10.0.0.1", "alice").is_ok(), "清除后应放行");
    }

    #[test]
    fn failure_counts_expire_after_window() {
        let mut t = LoginThrottle::new(2, 1); // 1 秒窗口
        t.record_failure("10.0.0.1", "alice");
        assert!(t.check("10.0.0.1", "alice").is_ok(), "1 次失败未达阈值");
        std::thread::sleep(Duration::from_millis(1100));
        t.record_failure("10.0.0.1", "alice");
        assert!(t.check("10.0.0.1", "alice").is_ok(), "首条记录已过期, 窗口内仅 1 次");
    }
}
