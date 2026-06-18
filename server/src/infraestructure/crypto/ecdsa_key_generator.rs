use p256::{
    SecretKey,
    pkcs8::{EncodePrivateKey, LineEnding},
};
use rand_core::OsRng;
use uuid::Uuid;

use crate::{
    application::ports::signing_key_generator::SigningKeyGenerator,
    domain::{
        entities::signing_key::{SigningAlgorithm, SigningKey},
        errors::DomainError,
    },
    utils::oauth_crypto::generate_opaque_token,
};

// Generates ECDSA P-256 (ES256) signing keys — small tokens, fast keygen, and
// supported by the `rust_crypto` backend of jsonwebtoken.
//
// The private key is exported as an unencrypted PKCS#8 PEM. The `SigningKey`
// field is named `private_key_encrypted` because production should seal it with a
// master key (or store a KMS reference); here, as elsewhere in this dev build, the
// raw PEM is stored and `TokenSigner` loads it directly.
pub struct EcdsaKeyGenerator;

impl EcdsaKeyGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EcdsaKeyGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SigningKeyGenerator for EcdsaKeyGenerator {
    fn generate(&self, tenant_id: Uuid) -> Result<SigningKey, DomainError> {
        let secret = SecretKey::random(&mut OsRng);

        let private_key_pem = secret
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| {
                DomainError::InfrastructureError(format!("failed to encode private key: {e}"))
            })?
            .to_string();

        // Public half as a JWK object: {"kty":"EC","crv":"P-256","x":..,"y":..}. The
        // /jwks.json endpoint stamps in `kid`, `alg`, and `use` when it serves this.
        let public_jwk = secret.public_key().to_jwk_string();

        // A random, opaque key id. Verifiers match it against the JWT header `kid`.
        let kid = generate_opaque_token();

        Ok(SigningKey::new(
            tenant_id,
            kid,
            SigningAlgorithm::ES256,
            public_jwk.to_string(),
            private_key_pem,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::token_signer::TokenSigner,
        infraestructure::crypto::jwt_signing_service::JwtSigningService,
    };
    use serde_json::json;

    #[test]
    fn generates_a_valid_es256_jwk() {
        let key = EcdsaKeyGenerator::new().generate(Uuid::now_v7()).unwrap();

        assert_eq!(key.algorithm().as_str(), "ES256");
        assert!(key.is_active());
        assert!(!key.kid().is_empty());

        // Public JWK is the EC public point, with no private scalar.
        let jwk: serde_json::Value = serde_json::from_str(key.public_jwk()).unwrap();
        assert_eq!(jwk["kty"], "EC");
        assert_eq!(jwk["crv"], "P-256");
        assert!(jwk.get("x").is_some() && jwk.get("y").is_some());
        assert!(
            jwk.get("d").is_none(),
            "private scalar must never appear in the public JWK"
        );
    }

    #[tokio::test]
    async fn the_generated_private_key_actually_signs() {
        let key = EcdsaKeyGenerator::new().generate(Uuid::now_v7()).unwrap();

        // The PEM must be loadable by the real signer — this exercises the whole
        // keygen → PEM → jsonwebtoken chain that the token endpoint depends on.
        let token = JwtSigningService::new()
            .sign(&key, &json!({ "sub": "abc", "exp": 9_999_999_999i64 }))
            .await
            .expect("generated key should produce a signable JWT");

        assert_eq!(
            token.split('.').count(),
            3,
            "a JWS has three dot-separated parts"
        );
    }

    // One-off ops backfill: provisions a signing key for every tenant that lacks an
    // active one. Tenants created before signing-key auto-provisioning have no key,
    // so the token endpoint 500s for them. Run against the live DB with:
    //
    //   cargo test --lib backfill_signing_keys_for_existing_tenants -- --ignored --nocapture
    //
    // Idempotent: tenants that already have an active key are skipped.
    #[tokio::test]
    #[ignore = "touches the live database; run manually as an ops step"]
    async fn backfill_signing_keys_for_existing_tenants() {
        use crate::{
            domain::repositories::{
                signing_key_repository::SigningKeyRepository, tenant_repository::TenantRepository,
            },
            infraestructure::database::repositories::{
                signing_key_repository::DbSigningKeyRepository,
                tenant_repository::DbTenantRepository,
            },
        };

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("connect to database");

        let tenants = DbTenantRepository::new(pool.clone());
        let signing_keys = DbSigningKeyRepository::new(pool.clone());
        let generator = EcdsaKeyGenerator::new();

        let all = tenants.find_all(1, 10_000).await.expect("list tenants");
        let mut provisioned = 0;
        for tenant in &all {
            if signing_keys
                .find_active(tenant.id())
                .await
                .expect("find_active")
                .is_some()
            {
                println!(
                    "tenant {} already has an active key — skipping",
                    tenant.id()
                );
                continue;
            }
            let key = generator.generate(tenant.id()).expect("generate key");
            signing_keys.add(&key).await.expect("persist key");
            provisioned += 1;
            println!(
                "provisioned key kid={} for tenant {}",
                key.kid(),
                tenant.id()
            );
        }
        println!(
            "done: {provisioned} key(s) provisioned across {} tenant(s)",
            all.len()
        );
    }

    // End-to-end proof, against the live DB, that the token endpoint now signs
    // instead of 500ing: provision a confirmed user, then run login → authorize →
    // token_exchange in-process and assert real JWTs come out.
    //
    //   cargo test --lib end_to_end_token_issuance -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "touches the live database; run manually to verify the token flow"]
    async fn end_to_end_token_issuance() {
        use std::sync::Arc;
        use uuid::Uuid;

        use crate::{
            application::{
                ports::unit_of_work::UnitOfWorkFactory,
                use_cases::{
                    authentication::{authorize_session, login},
                    oauth::token_exchange,
                },
            },
            domain::{
                entities::{
                    oauth_client::{ClientType, MfaPolicy, OAuthClient},
                    user::User,
                    user_credentials::UserCredentials,
                },
                repositories::{
                    oauth_client_repository::OAuthClientRepository,
                    tenant_repository::TenantRepository, user_repository::UserRepository,
                },
            },
            infraestructure::database::{
                repositories::{
                    oauth_client_repository::DbOAuthClientRepository,
                    tenant_repository::DbTenantRepository, user_repository::DbUserRepository,
                },
                unit_of_work::DbUnitOfWorkFactory,
            },
            utils::{oauth_crypto::pkce_challenge_s256, password_hasher::hash_password},
        };

        // Fixtures from the seeded dev data. We mint our own throwaway PUBLIC client so
        // the flow needs no client secret (the seeded client is confidential).
        const TENANT_ID: &str = "019e8e63-5c03-7b93-bfb2-2e32e8e4bba8";
        const REDIRECT_URI: &str = "http://localhost:5174/auth/callback";
        const PASSWORD: &str = "Passw0rd!end-to-end";
        let issuer = "http://localhost:8080".to_string();

        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("connect to database");

        let tenant_id = Uuid::parse_str(TENANT_ID).unwrap();
        let tenant = DbTenantRepository::new(pool.clone())
            .find_by_id(tenant_id)
            .await
            .unwrap()
            .expect("seed tenant exists");

        // A throwaway public client for this run.
        let public_client_id = Uuid::now_v7();
        let client = OAuthClient::new(
            tenant_id,
            public_client_id,
            None,
            ClientType::Public,
            "E2E Test Client".to_string(),
            vec![REDIRECT_URI.to_string()],
            vec![],
            vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            vec!["authorization_code".to_string()],
            true,
            false,
            MfaPolicy::default(),
        );
        DbOAuthClientRepository::new(pool.clone())
            .add(&client)
            .await
            .expect("insert test client");
        let client_id = public_client_id.to_string();

        // Provision a fresh, already-confirmed user with a known password.
        let email = format!("e2e-{}@test.local", Uuid::now_v7());
        let password_hash = hash_password(PASSWORD, tenant.password_hash_secret()).unwrap();
        let mut user = User::reconstruct(
            Uuid::now_v7(),
            email.clone(),
            tenant_id,
            true, // is_email_verified
            None,
            None,
            true, // is_active
            chrono::Utc::now(),
            None,
        );
        user.set_credentials(UserCredentials::new(
            user.id(),
            password_hash,
            "argon2".to_string(),
            false,
        ));
        DbUserRepository::new(pool.clone())
            .add(&user)
            .await
            .expect("insert test user");

        let uow_factory: Arc<dyn UnitOfWorkFactory> =
            Arc::new(DbUnitOfWorkFactory::new(pool.clone()));
        let signer: Arc<dyn TokenSigner> = Arc::new(JwtSigningService::new());

        // A real PKCE pair: keep the verifier, derive the challenge.
        let code_verifier = "e2e-code-verifier-0123456789-0123456789-0123456789";
        let code_challenge = pkce_challenge_s256(code_verifier);

        // Step 1: login → opaque session code.
        let login_uc = login::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::new(crate::testing::recording_email_sender::RecordingEmailSender::new()),
            Arc::new(crate::testing::fake_webauthn_service::FakeWebauthnService::new()),
        );
        let login_res = login_uc
            .execute(login::command::Command {
                email: email.clone(),
                password: PASSWORD.to_string(),
                client_id: client_id.clone(),
                redirect_uri: REDIRECT_URI.to_string(),
                response_type: "code".to_string(),
                scope: "openid email profile".to_string(),
                state: "e2e-state".to_string(),
                code_challenge: code_challenge.clone(),
                code_challenge_method: "S256".to_string(),
                nonce: Some("e2e-nonce".to_string()),
                mfa_methods: None,
                mfa_default_method: None,
            })
            .await
            .expect("login");

        // Step 2: authorize → single-use authorization code.
        let authz_uc = authorize_session::use_case::UseCase::new(Arc::clone(&uow_factory));
        let authz_res = authz_uc
            .execute(authorize_session::command::Command {
                session_code: login_res.session_code,
                client_id: client_id.clone(),
                email: email.clone(),
            })
            .await
            .expect("authorize");

        // Step 3: token exchange → the step that used to 500 for lack of a signing key.
        let token_uc = token_exchange::use_case::UseCase::new(
            Arc::clone(&uow_factory),
            Arc::clone(&signer),
            issuer,
            Arc::new(crate::testing::fake_audit_logger::FakeAuditLogger::new()),
        );
        let tokens = token_uc
            .execute(token_exchange::command::Command {
                tenant_id,
                client_id: client_id.clone(),
                client_secret: None,
                code: authz_res.authorization_code,
                redirect_uri: REDIRECT_URI.to_string(),
                code_verifier: code_verifier.to_string(),
            })
            .await
            .expect("token exchange should now succeed");

        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(
            tokens.access_token.split('.').count(),
            3,
            "access_token is a JWS"
        );
        assert!(
            tokens.id_token.is_some(),
            "openid scope must yield an id_token"
        );
        assert!(!tokens.refresh_token.is_empty());
        println!("✓ issued access_token + id_token for {email}");

        // Cleanup the throwaway user + client.
        DbUserRepository::new(pool.clone())
            .delete_by_id(user.id())
            .await
            .ok();
        DbOAuthClientRepository::new(pool.clone())
            .delete_by_id(tenant_id, client.id())
            .await
            .ok();
    }
}
