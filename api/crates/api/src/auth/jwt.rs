//! Access tokens: short-lived JWTs carrying `sub`, `tenant_id`, `role`, `jti`.
//! HS256 for development, RS256 when `JWT_ALGORITHM=RS256` with PEM keys.

use crate::config::JwtKeys;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use reelearn_domain::Role;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub tenant_id: Uuid,
    pub role: Role,
    pub jti: String,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("invalid key material: {0}")]
    Key(String),
    #[error("token could not be issued")]
    Encode,
    #[error("token is not valid")]
    Invalid,
    #[error("token has expired")]
    Expired,
}

pub struct Jwt {
    encoding: EncodingKey,
    decoding: DecodingKey,
    algorithm: Algorithm,
    issuer: String,
    ttl_seconds: i64,
}

impl std::fmt::Debug for Jwt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Jwt")
            .field("algorithm", &self.algorithm)
            .field("issuer", &self.issuer)
            .finish()
    }
}

impl Jwt {
    pub fn new(keys: &JwtKeys, issuer: &str, ttl_seconds: i64) -> Result<Jwt, JwtError> {
        let (encoding, decoding, algorithm) = match keys {
            JwtKeys::Hs256 { secret } => (
                EncodingKey::from_secret(secret.as_bytes()),
                DecodingKey::from_secret(secret.as_bytes()),
                Algorithm::HS256,
            ),
            JwtKeys::Rs256 {
                private_pem,
                public_pem,
            } => (
                EncodingKey::from_rsa_pem(private_pem.as_bytes())
                    .map_err(|e| JwtError::Key(e.to_string()))?,
                DecodingKey::from_rsa_pem(public_pem.as_bytes())
                    .map_err(|e| JwtError::Key(e.to_string()))?,
                Algorithm::RS256,
            ),
        };
        Ok(Jwt {
            encoding,
            decoding,
            algorithm,
            issuer: issuer.to_string(),
            ttl_seconds,
        })
    }

    pub fn ttl_seconds(&self) -> i64 {
        self.ttl_seconds
    }

    pub fn issue(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        role: Role,
    ) -> Result<(String, Claims), JwtError> {
        let now = Utc::now().timestamp();
        let claims = Claims {
            sub: user_id,
            tenant_id,
            role,
            jti: Uuid::now_v7().to_string(),
            iss: self.issuer.clone(),
            iat: now,
            exp: now + self.ttl_seconds,
        };
        let token = jsonwebtoken::encode(&Header::new(self.algorithm), &claims, &self.encoding)
            .map_err(|_| JwtError::Encode)?;
        Ok((token, claims))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.leeway = 5;
        match jsonwebtoken::decode::<Claims>(token, &self.decoding, &validation) {
            Ok(data) => Ok(data.claims),
            Err(e) if matches!(e.kind(), jsonwebtoken::errors::ErrorKind::ExpiredSignature) => {
                Err(JwtError::Expired)
            }
            Err(_) => Err(JwtError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hs256_round_trip_and_tamper_detection() {
        let jwt = Jwt::new(
            &JwtKeys::Hs256 {
                secret: "s".repeat(40),
            },
            "reelearn",
            900,
        )
        .expect("jwt");
        let (token, claims) = jwt
            .issue(Uuid::nil(), Uuid::max(), Role::Manager)
            .expect("issue");
        let back = jwt.verify(&token).expect("verify");
        assert_eq!(back.sub, Uuid::nil());
        assert_eq!(back.role, Role::Manager);
        assert_eq!(back.jti, claims.jti);
        let other = Jwt::new(
            &JwtKeys::Hs256 {
                secret: "x".repeat(40),
            },
            "reelearn",
            900,
        )
        .expect("jwt");
        assert!(matches!(other.verify(&token), Err(JwtError::Invalid)));
    }

    #[test]
    fn expired_tokens_are_rejected() {
        let jwt = Jwt::new(
            &JwtKeys::Hs256 {
                secret: "s".repeat(40),
            },
            "reelearn",
            -100,
        )
        .expect("jwt");
        let (token, _) = jwt
            .issue(Uuid::nil(), Uuid::nil(), Role::Learner)
            .expect("issue");
        assert!(matches!(jwt.verify(&token), Err(JwtError::Expired)));
    }
}
