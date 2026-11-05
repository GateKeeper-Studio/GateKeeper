#![forbid(unsafe_code)]

// CLI: repair rows whose at-rest secrets the CURRENT encryption key cannot read.
//
//   cargo run --bin repair_encrypted_secrets            # dry run: report only
//   cargo run --bin repair_encrypted_secrets -- --apply # write the fixes
//   cargo run --bin repair_encrypted_secrets -- --reseal
//       # upgrade every readable secret to the current format/key (the server also
//       # does this at startup); use it after rotating SECRETS_ENCRYPTION_KEY, then
//       # drop the old key from SECRETS_ENCRYPTION_PREVIOUS_KEYS.
//
// Secrets (tenant signing keys, TOTP seeds, SMTP passwords, webhook secrets, IdP
// client secrets, tenant peppers) are sealed with AES-256-GCM under
// SECRETS_ENCRYPTION_KEY (and any SECRETS_ENCRYPTION_PREVIOUS_KEYS). If that key
// changes — or was derived differently when the rows were written — every older row
// fails with "secret decryption failed" and surfaces as an opaque HTTP 500.
//
// Without the old key the plaintext is gone, so this tool only does what is possible
// without it:
//   - signing keys: retire an unreadable ACTIVE key and mint a fresh one, so the
//     tenant can sign tokens again (tokens issued under the old key stop verifying);
//   - TOTP enrollments: delete unreadable ones so the user can enroll again;
//   - SMTP passwords, webhook secrets, IdP client secrets: REPORT ONLY. An operator
//     has to re-enter the plaintext (dashboard / admin API).
//
// It reads the same .env as the API, so it must run with the same SECRETS_ENCRYPTION_KEY.

use std::sync::Arc;

use sqlx::{PgPool, Row};
use uuid::Uuid;
use web_api::{
    application::ports::{
        secret_cipher::{SecretCipher, SecretContext, SecretPurpose},
        signing_key_generator::SigningKeyGenerator,
    },
    domain::repositories::signing_key_repository::SigningKeyRepository,
    infraestructure::{
        config::AppConfig,
        crypto::{
            aes_gcm_secret_cipher::AesGcmSecretCipher, ecdsa_key_generator::EcdsaKeyGenerator,
            secret_reseal::reseal_legacy_secrets,
        },
        database::{
            database_pool::create_database_pool,
            repositories::signing_key_repository::DbSigningKeyRepository,
        },
    },
};

#[tokio::main]
async fn main() {
    let apply = std::env::args().skip(1).any(|a| a == "--apply");
    let reseal = std::env::args().skip(1).any(|a| a == "--reseal");

    let config = AppConfig::from_env();
    let pool = create_database_pool(&config.database_url, &config.database).await;
    let cipher: Arc<dyn SecretCipher> = Arc::new(AesGcmSecretCipher::from_config(&config));

    if reseal {
        match reseal_legacy_secrets(&pool, cipher.as_ref()).await {
            Ok(report) => println!(
                "re-sealed {} secret(s); {} unreadable (run without --reseal to inspect)",
                report.resealed, report.unreadable
            ),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    eprintln!(
        "# mode: {}",
        if apply {
            "APPLY (writing)"
        } else {
            "dry run (pass --apply to write)"
        }
    );

    if let Err(e) = run(&pool, cipher, apply).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(pool: &PgPool, cipher: Arc<dyn SecretCipher>, apply: bool) -> Result<(), String> {
    repair_signing_keys(pool, &cipher, apply).await?;
    repair_totp(pool, &cipher, apply).await?;
    report_only(
        pool,
        &cipher,
        "tenant_emails",
        "SELECT id, tenant_id, password_encrypted AS enc FROM tenant_emails",
        SecretPurpose::SmtpPassword,
        "SMTP password — re-save the email provider in the dashboard",
    )
    .await?;
    report_only(
        pool,
        &cipher,
        "webhook_subscriptions",
        "SELECT id, tenant_id, secret_encrypted AS enc FROM webhook_subscriptions",
        SecretPurpose::WebhookSecret,
        "webhook signing secret — rotate/re-create the subscription",
    )
    .await?;
    report_only(
        pool,
        &cipher,
        "external_identity_providers",
        "SELECT id, tenant_id, provider_client_secret_encrypted AS enc FROM external_identity_providers",
        SecretPurpose::ExternalIdpClientSecret,
        "IdP client secret — re-save the provider with its client secret",
    )
    .await?;
    Ok(())
}

async fn repair_signing_keys(
    pool: &PgPool,
    cipher: &Arc<dyn SecretCipher>,
    apply: bool,
) -> Result<(), String> {
    let repo = DbSigningKeyRepository::new(pool.clone());
    let generator = EcdsaKeyGenerator::new(Arc::clone(cipher));

    let tenants: Vec<Uuid> = sqlx::query("SELECT id FROM tenants")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| r.get("id"))
        .collect();

    for tenant_id in tenants {
        let active = repo
            .find_active(tenant_id)
            .await
            .map_err(|e| e.to_string())?;
        match active {
            Some(mut key)
                if cipher
                    .decrypt(
                        key.private_key_encrypted(),
                        SecretContext::new(SecretPurpose::SigningKey, tenant_id),
                    )
                    .is_err() =>
            {
                println!(
                    "signing key  tenant={tenant_id} kid={}: UNREADABLE -> replace",
                    key.kid()
                );
                if apply {
                    // Retired (not deleted): out of the JWKS, row kept for the audit trail.
                    key.retire();
                    repo.update(&key).await.map_err(|e| e.to_string())?;
                    let fresh = generator.generate(tenant_id).map_err(|e| e.to_string())?;
                    repo.add(&fresh).await.map_err(|e| e.to_string())?;
                    println!("             minted new key kid={}", fresh.kid());
                }
            }
            Some(key) => println!("signing key  tenant={tenant_id} kid={}: ok", key.kid()),
            None => {
                println!("signing key  tenant={tenant_id}: NO ACTIVE KEY -> create");
                if apply {
                    let fresh = generator.generate(tenant_id).map_err(|e| e.to_string())?;
                    repo.add(&fresh).await.map_err(|e| e.to_string())?;
                    println!("             minted new key kid={}", fresh.kid());
                }
            }
        }
    }
    Ok(())
}

async fn repair_totp(
    pool: &PgPool,
    cipher: &Arc<dyn SecretCipher>,
    apply: bool,
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, user_id, secret_enc FROM mfa_enrollments \
         WHERE method = 'totp' AND secret_enc IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for row in rows {
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let user_id: Uuid = row.get("user_id");
        let enc: String = row.get("secret_enc");
        let context = SecretContext::new(SecretPurpose::TotpSeed, tenant_id);
        if cipher.decrypt(&enc, context).is_ok() {
            println!("totp         enrollment={id} user={user_id}: ok");
            continue;
        }
        println!(
            "totp         enrollment={id} user={user_id}: UNREADABLE -> delete (user re-enrolls)"
        );
        if apply {
            sqlx::query("DELETE FROM mfa_enrollments WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// `sql` is a static literal, so no dynamic SQL is built.
async fn report_only(
    pool: &PgPool,
    cipher: &Arc<dyn SecretCipher>,
    table: &str,
    sql: &'static str,
    purpose: SecretPurpose,
    advice: &str,
) -> Result<(), String> {
    let rows = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let enc: String = row.get("enc");
        if cipher
            .decrypt(&enc, SecretContext::new(purpose, tenant_id))
            .is_ok()
        {
            println!("{table:<13} id={id}: ok");
        } else {
            println!("{table:<13} id={id}: UNREADABLE (manual) — {advice}");
        }
    }
    Ok(())
}
