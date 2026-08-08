/// 校验明文密码与存储哈希是否匹配。
/// 存储格式统一为服务端 bcrypt(明文密码)，随机盐。不保留任何历史客户端哈希兼容逻辑。
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    bcrypt::verify(password, stored_hash).unwrap_or(false)
}

pub fn hash_password(password: &str, cost: u32) -> Result<String, crate::error::AppError> {
    bcrypt::hash(password, cost)
        .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("Bcrypt error: {}", e)))
}

/// F-02: 生成一次性随机初始密码（16 位，字母+数字）。
/// 由 uuid v4 的随机字节映射到 62 字符集，约 95 bit 熵；仅用于创建员工时的初始凭据，
/// 配合 must_change_password 标记强制首次登录改密，不承担长期口令强度职责。
pub fn generate_random_password() -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let bytes = uuid::Uuid::new_v4().as_bytes().to_vec();
    let mut out = String::with_capacity(16);
    for b in bytes.iter().take(16) {
        out.push(CHARS[(*b as usize) % CHARS.len()] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_correct_password() {
        let stored = hash_password("secret123", 4).unwrap();
        assert!(verify_password("secret123", &stored));
    }

    #[test]
    fn reject_wrong_password() {
        let stored = hash_password("secret123", 4).unwrap();
        assert!(!verify_password("wrong", &stored));
    }

    #[test]
    fn reject_empty_stored() {
        assert!(!verify_password("secret123", ""));
    }
}
