// Manner_Web - 可以在 Linux 系统上运行的企业管理系统
// Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 可信反向代理 IP 白名单（支持单 IP 与 CIDR）。
//!
//! 安全前提：仅当请求的 TCP 对端地址命中白名单时，后端才信任
//! `X-Real-IP` / `X-Forwarded-For` 提供的真实客户端 IP，用于登录限流与审计日志溯源；
//! 直连部署（无代理）时白名单为空，后端忽略一切转发头，杜绝伪造 XFF 绕过限流。
//! 经反向代理部署时应配置 `TRUSTED_PROXIES=127.0.0.1,10.0.0.0/8` 等可信网段。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Clone)]
enum Entry {
    Exact(IpAddr),
    Cidr4 { base: Ipv4Addr, prefix: u8 },
    Cidr6 { base: Ipv6Addr, prefix: u8 },
}

/// 可信代理白名单。空 = 不信任任何转发头（默认直连部署）。
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    entries: Vec<Entry>,
}

impl TrustedProxies {
    /// 从逗号分隔字符串解析：`127.0.0.1,10.0.0.0/8,::1`。
    /// 非法条目静默忽略，不使整个配置失效。
    pub fn parse(list: &str) -> Self {
        let mut entries = Vec::new();
        for part in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some((ip_str, prefix_str)) = part.split_once('/') {
                let prefix: u8 = match prefix_str.parse() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    if prefix <= 32 {
                        entries.push(Entry::Cidr4 { base: ip, prefix });
                        continue;
                    }
                }
                if let Ok(ip) = ip_str.parse::<Ipv6Addr>() {
                    if prefix <= 128 {
                        entries.push(Entry::Cidr6 { base: ip, prefix });
                        continue;
                    }
                }
            } else if let Ok(ip) = part.parse::<IpAddr>() {
                entries.push(Entry::Exact(ip));
            }
        }
        Self { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, ip: &IpAddr) -> bool {
        self.entries.iter().any(|e| match e {
            Entry::Exact(x) => x == ip,
            Entry::Cidr4 { base, prefix } => match ip {
                IpAddr::V4(v4) => {
                    let shift = 32 - prefix;
                    let mask = if shift == 32 { 0 } else { u32::MAX << shift };
                    (u32::from(*base) & mask) == (u32::from(*v4) & mask)
                }
                _ => false,
            },
            Entry::Cidr6 { base, prefix } => match ip {
                IpAddr::V6(v6) => {
                    let a = base.octets();
                    let b = v6.octets();
                    let full_bytes = (prefix / 8) as usize;
                    let rem_bits = prefix % 8;
                    if a[..full_bytes] != b[..full_bytes] {
                        return false;
                    }
                    if rem_bits == 0 {
                        true
                    } else {
                        let mask = 0xFFu8 << (8 - rem_bits);
                        a[full_bytes] & mask == b[full_bytes] & mask
                    }
                }
                _ => false,
            },
        })
    }
}

/// 解析真实客户端 IP：对端地址命中可信代理白名单时，读取转发头；否则以 TCP 对端为准。
///
/// - 优先级：`X-Real-IP` → `X-Forwarded-For` 首个值；
/// - 提取出的客户端 IP 若本身落在代理白名单内则忽略（防递归/自指伪造）；
/// - 无代理/白名单为空时一律返回对端地址（不信任任何转发头）。
pub fn resolve_client_ip(
    peer: Option<SocketAddr>,
    headers: &axum::http::HeaderMap,
    trusted: &TrustedProxies,
) -> String {
    let peer_ip = peer.map(|a| a.ip()).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    if trusted.is_empty() || !trusted.contains(&peer_ip) {
        return peer_ip.to_string();
    }
    for header in ["x-real-ip", "x-forwarded-for"] {
        if let Some(raw) = headers.get(header).and_then(|h| h.to_str().ok()) {
            let candidate = if header == "x-forwarded-for" {
                raw.split(',').next().map(str::trim).unwrap_or("")
            } else {
                raw.trim()
            };
            if let Ok(ip) = candidate.parse::<IpAddr>() {
                if !trusted.contains(&ip) {
                    return ip.to_string();
                }
            }
        }
    }
    peer_ip.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_trust_never_accepts_forwarded() {
        let t = TrustedProxies::parse("");
        assert!(t.is_empty());
        assert!(!t.contains(&"127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn exact_ip_and_cidr_match() {
        let t = TrustedProxies::parse("127.0.0.1,10.0.0.0/8,::1");
        assert!(t.contains(&"127.0.0.1".parse().unwrap()));
        assert!(t.contains(&"10.1.2.3".parse().unwrap()));
        assert!(!t.contains(&"11.0.0.1".parse().unwrap()));
        assert!(t.contains(&"::1".parse().unwrap()));
        assert!(!t.contains(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn cidr_boundaries() {
        let t = TrustedProxies::parse("10.0.0.0/8,192.168.1.0/24");
        assert!(t.contains(&"10.255.255.255".parse().unwrap()));
        assert!(!t.contains(&"11.0.0.0".parse().unwrap()));
        assert!(t.contains(&"192.168.1.0".parse().unwrap()));
        assert!(t.contains(&"192.168.1.255".parse().unwrap()));
        assert!(!t.contains(&"192.168.2.0".parse().unwrap()));
    }

    #[test]
    fn invalid_entries_ignored() {
        let t = TrustedProxies::parse("999.1.1.1,bogus,10.0.0.0/99");
        assert!(t.is_empty());
    }

    #[test]
    fn resolve_ignores_forwarded_when_untrusted_peer() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "8.8.8.8".parse().unwrap());
        let t = TrustedProxies::parse("");
        let peer: SocketAddr = "1.2.3.4:5000".parse().unwrap();
        assert_eq!(
            resolve_client_ip(Some(peer), &headers, &t),
            "1.2.3.4".to_string()
        );
    }

    #[test]
    fn resolve_accepts_forwarded_from_trusted_proxy() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.9".parse().unwrap());
        headers.insert("x-forwarded-for", "203.0.113.9, 127.0.0.1".parse().unwrap());
        let t = TrustedProxies::parse("127.0.0.1");
        let peer: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        // 优先 X-Real-IP
        assert_eq!(
            resolve_client_ip(Some(peer), &headers, &t),
            "203.0.113.9".to_string()
        );
    }

    #[test]
    fn resolve_rejects_self_referential_proxy_ip() {
        let mut headers = axum::http::HeaderMap::new();
        // 攻击者伪造 X-Real-IP 指向代理自身 → 回退到对端
        headers.insert("x-real-ip", "127.0.0.1".parse().unwrap());
        let t = TrustedProxies::parse("127.0.0.1");
        let peer: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        assert_eq!(
            resolve_client_ip(Some(peer), &headers, &t),
            "127.0.0.1".to_string()
        );
    }
}
