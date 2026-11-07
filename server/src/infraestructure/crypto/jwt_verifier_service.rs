use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::Jwk};

use crate::{
    application::ports::token_verifier::{TokenVerifier, VerifiedToken},
    domain::entities::signing_key::{SigningAlgorithm, SigningKey},
    domain::errors::DomainError,
};

// Verifies JWTs against a tenant's PUBLISHED keys, selecting the right one by the
// `kid` in the token header. Asymmetric only — an `alg=none` or HMAC token is
// rejected because we only ever build a DecodingKey from a public JWK. The
// expected `issuer` is also enforced so a token minted by some other provider (or
// for a different issuer) can never be replayed here.
pub struct JwtVerifierService {
    issuer: String,
}

impl JwtVerifierService {
    pub fn new(issuer: String) -> Self {
        Self { issuer }
    }
}

fn algorithm_of(key: &SigningKey) -> Algorithm {
    match key.algorithm() {
        SigningAlgorithm::RS256 => Algorithm::RS256,
        SigningAlgorithm::ES256 => Algorithm::ES256,
    }
}

// The published key a token's `kid` names, as a DecodingKey.
fn decoding_key_for(
    kid: Option<String>,
    keys: &[SigningKey],
) -> Result<(DecodingKey, Algorithm), DomainError> {
    let kid = kid.ok_or_else(|| DomainError::AccessDenied("token missing kid".to_string()))?;

    let key = keys.iter().find(|k| k.kid() == kid).ok_or_else(|| {
        DomainError::AccessDenied("no published key matches token kid".to_string())
    })?;

    let jwk: Jwk = serde_json::from_str(key.public_jwk()).map_err(|_| {
        DomainError::InfrastructureError("stored JWK is not valid JSON".to_string())
    })?;
    let decoding_key = DecodingKey::from_jwk(&jwk)
        .map_err(|e| DomainError::InfrastructureError(format!("invalid JWK: {e}")))?;
    Ok((decoding_key, algorithm_of(key)))
}

fn verified(claims: serde_json::Value) -> VerifiedToken {
    let subject = claims
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let scopes = claims
        .get("scope")
        .and_then(|v| v.as_str())
        .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
        .unwrap_or_default();

    VerifiedToken {
        subject,
        scopes,
        claims,
    }
}

#[async_trait]
impl TokenVerifier for JwtVerifierService {
    async fn verify_id_token_hint(
        &self,
        token: &str,
        keys: &[SigningKey],
    ) -> Result<VerifiedToken, DomainError> {
        let header = decode_header(token)
            .map_err(|_| DomainError::AccessDenied("malformed token header".to_string()))?;

        // Only an ID token is a valid hint. Our ID tokens are `typ: JWT`; an access
        // token (`at+jwt`) or a logout token (`logout+jwt`) is refused.
        if !header
            .typ
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("JWT"))
        {
            return Err(DomainError::AccessDenied(
                "token is not an ID token".to_string(),
            ));
        }

        let (decoding_key, algorithm) = decoding_key_for(header.kid, keys)?;

        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["iss", "sub", "aud"]);
        // RP-Initiated Logout 1.0 §2: an expired ID token is still a valid hint.
        validation.validate_exp = false;
        // The caller compares `aud` with the client that is signing the user out.
        validation.validate_aud = false;

        let decoded = decode::<serde_json::Value>(token, &decoding_key, &validation)
            .map_err(|_| DomainError::AccessDenied("invalid id_token_hint".to_string()))?;

        Ok(verified(decoded.claims))
    }

    async fn verify(&self, token: &str, keys: &[SigningKey]) -> Result<VerifiedToken, DomainError> {
        let header = decode_header(token)
            .map_err(|_| DomainError::AccessDenied("malformed token header".to_string()))?;

        // Only access tokens are bearer credentials. `application/at+jwt` is the
        // media-type spelling RFC 9068 also allows.
        let is_access_token = header.typ.as_deref().is_some_and(|t| {
            t.eq_ignore_ascii_case("at+jwt") || t.eq_ignore_ascii_case("application/at+jwt")
        });
        if !is_access_token {
            return Err(DomainError::AccessDenied(
                "token is not an access token".to_string(),
            ));
        }

        let (decoding_key, algorithm) = decoding_key_for(header.kid, keys)?;

        let mut validation = Validation::new(algorithm);
        // `exp` is validated by default. Pin the issuer so cross-issuer tokens are
        // rejected; require `exp` to be present so a token without it can't slip through.
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss"]);
        // The resource server (this endpoint) validates the audience itself, not here.
        validation.validate_aud = false;

        let decoded = decode::<serde_json::Value>(token, &decoding_key, &validation)
            .map_err(|_| DomainError::AccessDenied("invalid_token".to_string()))?;

        Ok(verified(decoded.claims))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::{
        application::ports::{
            secret_cipher::SecretCipher,
            signing_key_generator::SigningKeyGenerator,
            token_signer::{TokenSigner, TokenType},
        },
        infraestructure::crypto::{
            aes_gcm_secret_cipher::AesGcmSecretCipher, ecdsa_key_generator::EcdsaKeyGenerator,
            jwt_signing_service::JwtSigningService,
        },
    };

    const ISSUER: &str = "https://idp.example.com";

    struct Keys {
        signer: JwtSigningService,
        key: SigningKey,
    }

    fn keys() -> Keys {
        let cipher: Arc<dyn SecretCipher> = Arc::new(AesGcmSecretCipher::new([7u8; 32]));
        let key = EcdsaKeyGenerator::new(Arc::clone(&cipher))
            .generate(Uuid::now_v7())
            .unwrap();
        Keys {
            signer: JwtSigningService::new(cipher),
            key,
        }
    }

    fn id_claims(exp_offset: i64) -> serde_json::Value {
        let now = Utc::now().timestamp();
        json!({
            "iss": ISSUER,
            "sub": Uuid::now_v7().to_string(),
            "aud": "client-1",
            "sid": Uuid::now_v7().to_string(),
            "iat": now - 7200,
            "exp": now + exp_offset,
        })
    }

    async fn sign(keys: &Keys, claims: &serde_json::Value, typ: TokenType) -> String {
        keys.signer.sign(&keys.key, claims, typ).await.unwrap()
    }

    // RP-Initiated Logout: an expired ID token is still a good hint.
    #[tokio::test]
    async fn an_expired_id_token_is_a_valid_hint() {
        let keys = keys();
        let claims = id_claims(-3600);
        let token = sign(&keys, &claims, TokenType::Id).await;

        let verified = JwtVerifierService::new(ISSUER.to_string())
            .verify_id_token_hint(&token, std::slice::from_ref(&keys.key))
            .await
            .expect("expired, but signed by us");

        assert_eq!(verified.claims["sid"], claims["sid"]);
        assert_eq!(verified.subject, claims["sub"].as_str().unwrap());
    }

    // Only an ID token is a hint; and only an access token is a bearer credential —
    // a logout token is neither.
    #[tokio::test]
    async fn each_token_type_is_accepted_only_where_it_belongs() {
        let keys = keys();
        let verifier = JwtVerifierService::new(ISSUER.to_string());
        let claims = id_claims(600);
        let published = std::slice::from_ref(&keys.key);

        for typ in [TokenType::Access, TokenType::Logout] {
            let token = sign(&keys, &claims, typ).await;
            assert!(
                verifier
                    .verify_id_token_hint(&token, published)
                    .await
                    .is_err(),
                "{typ:?} must not pass as an id_token_hint"
            );
        }
        for typ in [TokenType::Id, TokenType::Logout] {
            let token = sign(&keys, &claims, typ).await;
            assert!(
                verifier.verify(&token, published).await.is_err(),
                "{typ:?} must not pass as an access token"
            );
        }
    }

    #[tokio::test]
    async fn a_hint_from_another_issuer_or_key_is_refused() {
        let keys = keys();
        let mut claims = id_claims(600);
        claims["iss"] = json!("https://other-idp.example.com");
        let token = sign(&keys, &claims, TokenType::Id).await;
        let verifier = JwtVerifierService::new(ISSUER.to_string());

        assert!(
            verifier
                .verify_id_token_hint(&token, std::slice::from_ref(&keys.key))
                .await
                .is_err()
        );

        // Signed by a key that is not published for the tenant.
        let token = sign(&keys, &id_claims(600), TokenType::Id).await;
        let unrelated = self::keys().key;
        assert!(
            verifier
                .verify_id_token_hint(&token, std::slice::from_ref(&unrelated))
                .await
                .is_err()
        );
    }
}
