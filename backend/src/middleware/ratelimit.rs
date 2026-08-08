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

/// 登录失败节流器：按「真实 IP」与「用户名」双维度计数。
///
/// - 键由调用方传入真实 TCP 对端地址（不信任 X-Forwarded-For 等可伪造头）。
/// - 任一维度在时间窗口内失败次数达到阈值即锁定该维度，窗口过期后自动复位。
/// - 无全局锁：攻击者无法用随机凭据锁死全站登录（避免全局锁定 DoS 放大）。
/// - 限流参数（max_failures / window）通过系统设置动态调整，`update_limits` 立即生效。
pub struct LoginThrottle {
    map: HashMap<String, Vec<Instant>>,
    limits: std::sync::RwLock<LoginLimits>,
}

impl LoginThrottle {
    pub fn new(max_failures: usize, window_secs: u64) -> Self {
        Self {
            map: HashMap::new(),
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
        self.map.clear();
    }

    /// 清理窗口外的时间戳；空条目一并移除，防止 map 无限增长。
    fn prune(&mut self, now: Instant) {
        let window = Duration::from_secs(self.current_limits().lock_window_secs);
        self.map.retain(|_, times| {
            times.retain(|t| now.duration_since(*t) <= window);
            !times.is_empty()
        });
    }

    fn locked_retry_after(&self, key: &str, now: Instant) -> Option<u64> {
        let limits = self.current_limits();
        let window = Duration::from_secs(limits.lock_window_secs);
        let times = self.map.get(key)?;
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
        if let Some(secs) = self
            .locked_retry_after(&format!("ip:{ip}"), now)
            .or_else(|| self.locked_retry_after(&format!("user:{username}"), now))
        {
            return Err(AppError::TooManyRequests {
                retry_after_secs: secs,
            });
        }
        Ok(())
    }

    /// 记录一次登录失败（IP 与用户名双维度同时计数）。
    pub fn record_failure(&mut self, ip: &str, username: &str) {
        let now = Instant::now();
        self.prune(now);
        for key in [format!("ip:{ip}"), format!("user:{username}")] {
            self.map.entry(key).or_default().push(now);
        }
    }

    /// 登录成功时清除该 IP 与该用户名的失败记录。
    pub fn clear(&mut self, ip: &str, username: &str) {
        self.map.remove(&format!("ip:{ip}"));
        self.map.remove(&format!("user:{username}"));
    }
}

pub type SharedLoginThrottle = Arc<Mutex<LoginThrottle>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_after_max_failures_on_either_dimension() {
        let mut t = LoginThrottle::new(3, 900);
        // 仅用户名维度失败达到阈值 → 锁定(不依赖 IP)
        t.record_failure("10.0.0.1", "alice");
        t.record_failure("10.0.0.2", "alice");
        t.record_failure("10.0.0.3", "alice");
        assert!(t.check("10.0.0.9", "alice").is_err(), "user 维度应锁定");

        // 另一组:仅 IP 维度失败达到阈值 → 锁定
        let mut t2 = LoginThrottle::new(3, 900);
        t2.record_failure("10.0.0.1", "bob");
        t2.record_failure("10.0.0.1", "carol");
        t2.record_failure("10.0.0.1", "dave");
        assert!(t2.check("10.0.0.1", "zzz_new_user").is_err(), "ip 维度应锁定");
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
