//! 员工敏感字段静态加密（AES-256-GCM）。
//!
//! - 密文格式：`enc:v1:<base64(nonce || ciphertext)>`。
//!   `enc:v1:` 前缀用于幂等迁移检测：启动迁移时仅对「无前缀」的存量明文做加密，
//!   已带前缀的字段跳过——即使 `FIELD_ENC_KEY` 被轮换，也不会把旧密文当明文
//!   二次加密导致数据损坏（轮换本身是破坏性操作，见 docs/加密标准.md）。
//! - 密钥由环境变量 `FIELD_ENC_KEY` 经 SHA-256 派生为 32 字节（AES-256），
//!   对配置长度不敏感；未配置时服务拒绝启动（见 config.rs）。
//! - 所有业务 API 一律返回脱敏值（全掩 `***`），密文不离开数据库；
//!   明文仅通过受权限控制 + 强制日志的解密 API（employee:view_sensitive）获取。
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;

use crate::error::AppError;

/// 密文前缀：用于识别「已是密文」的字段（幂等迁移 + 密钥轮换保护）。
pub const ENC_PREFIX: &str = "enc:v1:";
/// AES-GCM 推荐 nonce 长度（96 bit）。
const NONCE_LEN: usize = 12;

/// 从任意字符串派生 32 字节 AES-256 密钥（SHA-256）。
pub fn derive_key(secret: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

/// 加密明文字符串，返回带 `enc:v1:` 前缀的密文。
pub fn encrypt_field(plain: &str, key: &[u8; 32]) -> Result<String, AppError> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plain.as_bytes())
        .map_err(|_| AppError::Internal(anyhow::anyhow!("AES-GCM 加密失败")))?;
    let mut raw = nonce.to_vec();
    raw.extend_from_slice(&ct);
    Ok(format!("{}{}", ENC_PREFIX, BASE64.encode(raw)))
}

/// 尝试解密带前缀的密文。
///
/// - 字段无 `enc:v1:` 前缀（存量明文或空值）→ 返回 `Ok(None)`；
/// - 前缀正确但解密失败（密钥不匹配 / 数据损坏）→ 返回 Err（不静默吞掉，
///   避免「旧密文被当明文二次加密」的隐性损坏，交由上层记录日志）。
pub fn try_decrypt_field(encoded: &str, key: &[u8; 32]) -> Result<Option<String>, AppError> {
    let body = match encoded.strip_prefix(ENC_PREFIX) {
        Some(b) => b,
        None => return Ok(None),
    };
    let raw = BASE64
        .decode(body)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("敏感字段密文解码失败")))?;
    if raw.len() < NONCE_LEN {
        return Err(AppError::Internal(anyhow::anyhow!(
            "敏感字段密文长度非法"
        )));
    }
    let (nonce, ct) = raw.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(key.into());
    let pt = cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| AppError::Internal(anyhow::anyhow!(
            "敏感字段密文解密失败（FIELD_ENC_KEY 不匹配或数据损坏）"
        )))?;
    let s = String::from_utf8(pt)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("敏感字段明文非法 UTF-8")))?;
    Ok(Some(s))
}

/// 全掩脱敏：字段有值 → `***`，无值 → `None`。用于所有业务 API 返回。
pub fn mask_field(v: Option<String>) -> Option<String> {
    v.map(|_| "***".to_string())
}

/// 启动时幂等迁移：将存量明文敏感字段加密回写为密文。
///
/// 判定规则：字段非空且不以 `enc:v1:` 开头 → 视为明文，加密回写；
/// 已带前缀 → 跳过（幂等）；解密失败 → 记录 WARN 后跳过（不覆盖，避免损坏）。
/// 空字符串统一转为 NULL。
pub async fn migrate_sensitive_fields(
    pool: &sqlx::MySqlPool,
    key: &[u8; 32],
) -> Result<usize, AppError> {
    let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT id, email, phone, id_number, address FROM employees \
             WHERE email IS NOT NULL OR phone IS NOT NULL OR id_number IS NOT NULL OR address IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;

    let mut migrated = 0usize;
    for (id, email, phone, id_number, address) in rows {
        let new_email = migrate_one(email, key);
        let new_phone = migrate_one(phone, key);
        let new_id_number = migrate_one(id_number, key);
        let new_address = migrate_one(address, key);

        if new_email.is_none()
            && new_phone.is_none()
            && new_id_number.is_none()
            && new_address.is_none()
        {
            continue;
        }

        // 覆盖写回：已加密/无需迁移的字段仍写回原值，保持幂等。
        sqlx::query(
            "UPDATE employees SET email = ?, phone = ?, id_number = ?, address = ? WHERE id = ?",
        )
        .bind(&new_email)
        .bind(&new_phone)
        .bind(&new_id_number)
        .bind(&new_address)
        .bind(&id)
        .execute(pool)
        .await?;
        migrated += 1;
    }
    Ok(migrated)
}

/// 单个字段迁移：明文 → 密文；已加密/空 → 保持。
fn migrate_one(value: Option<String>, key: &[u8; 32]) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with(ENC_PREFIX) {
        return Some(value);
    }
    match encrypt_field(trimmed, key) {
        Ok(ct) => Some(ct),
        Err(e) => {
            tracing::warn!(error = ?e, "敏感字段加密迁移失败，保留原值");
            Some(value)
        }
    }
}
