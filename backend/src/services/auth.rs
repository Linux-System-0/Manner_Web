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

/// 校验明文密码与存储哈希是否匹配。
/// 存储格式统一为服务端 bcrypt(明文密码)，随机盐。不保留任何历史客户端哈希兼容逻辑。
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    bcrypt::verify(password, stored_hash).unwrap_or(false)
}

/// 最小密码长度（注册/修改/重置/首登激活统一执行）。
pub const MIN_PASSWORD_LEN: usize = 8;
/// bcrypt 有效输入上限为 72 字节；超长密码在多数实现中被静默截断，
/// 显式拒绝以消除「截断后等价密码」的歧义并避免无效计算。
pub const MAX_PASSWORD_BYTES: usize = 72;

/// 统一密码强度校验：8~72 字节。
pub fn validate_password_strength(
    password: &str,
) -> Result<(), crate::error::AppError> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_LEN {
        return Err(crate::error::AppError::ValidationError(
            format!("密码至少 {} 位", MIN_PASSWORD_LEN),
        ));
    }
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(crate::error::AppError::ValidationError(
            format!("密码不能超过 {} 字节", MAX_PASSWORD_BYTES),
        ));
    }
    Ok(())
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
