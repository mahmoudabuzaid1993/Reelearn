//! Hashing and encryption helpers: argon2 for OTP codes, SHA-256 for refresh tokens,
//! AES-256-GCM for provider secrets in `integrations.config_enc`.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng as AeadOsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("hashing failed")]
    Hash,
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
    #[error("key is not 32 bytes")]
    Key,
}

pub fn hash_otp(code: &str) -> Result<String, CryptoError> {
    let salt = SaltString::generate(&mut OsRng);
    // Light parameters: codes expire in minutes and are rate limited; sign-in must stay fast on a phone.
    let params = argon2::Params::new(8 * 1024, 2, 1, None).map_err(|_| CryptoError::Hash)?;
    let argon = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    Ok(argon
        .hash_password(code.as_bytes(), &salt)
        .map_err(|_| CryptoError::Hash)?
        .to_string())
}

pub fn verify_otp(code: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(code.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn sha256_hex(input: &[u8]) -> String {
    hex::encode(Sha256::digest(input))
}

/// 32 random bytes as base64url (refresh tokens, upload ids).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn random_otp_code() -> String {
    let n = rand::thread_rng().next_u32() % 1_000_000;
    format!("{n:06}")
}

#[derive(Clone)]
pub struct SecretBox {
    key: [u8; 32],
}

impl std::fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBox(..)")
    }
}

impl SecretBox {
    pub fn from_hex(hex_key: &str) -> Result<SecretBox, CryptoError> {
        let bytes = hex::decode(hex_key).map_err(|_| CryptoError::Key)?;
        let key: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::Key)?;
        Ok(SecretBox { key })
    }

    /// nonce(12) || ciphertext
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| CryptoError::Key)?;
        let nonce = Aes256Gcm::generate_nonce(&mut AeadOsRng);
        let ct = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| CryptoError::Encrypt)?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if blob.len() < 13 {
            return Err(CryptoError::Decrypt);
        }
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| CryptoError::Key)?;
        let (nonce, ct) = blob.split_at(12);
        cipher
            .decrypt(Nonce::from_slice(nonce), ct)
            .map_err(|_| CryptoError::Decrypt)
    }

    pub fn encrypt_json(&self, value: &serde_json::Value) -> Result<Vec<u8>, CryptoError> {
        let bytes = serde_json::to_vec(value).map_err(|_| CryptoError::Encrypt)?;
        self.encrypt(&bytes)
    }

    pub fn decrypt_json(&self, blob: &[u8]) -> Result<serde_json::Value, CryptoError> {
        let bytes = self.decrypt(blob)?;
        serde_json::from_slice(&bytes).map_err(|_| CryptoError::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_hash_round_trip() {
        let h = hash_otp("1234").expect("hash");
        assert!(verify_otp("1234", &h));
        assert!(!verify_otp("1235", &h));
        assert!(!verify_otp("1234", "garbage"));
    }

    #[test]
    fn secret_box_round_trip() {
        let sb = SecretBox::from_hex(&"ab".repeat(32)).expect("key");
        let v = serde_json::json!({"api_key": "sk-123", "phone_number_id": "42"});
        let blob = sb.encrypt_json(&v).expect("enc");
        assert_ne!(blob, serde_json::to_vec(&v).expect("json"));
        assert_eq!(sb.decrypt_json(&blob).expect("dec"), v);
        let other = SecretBox::from_hex(&"cd".repeat(32)).expect("key");
        assert!(other.decrypt_json(&blob).is_err());
        assert!(SecretBox::from_hex("short").is_err());
    }

    #[test]
    fn tokens_are_random_and_hashable() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(sha256_hex(b"x").len(), 64);
        assert_eq!(random_otp_code().len(), 6);
    }
}
