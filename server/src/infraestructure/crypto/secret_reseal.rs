// Upgrades every secret at rest to the current sealing format and key.
//
// Runs once at startup (after migrations) and from
// `cargo run --bin repair_encrypted_secrets -- --reseal`. For each sealed column it
// rewrites values that are:
//   - in the legacy envelope (no key id, no associated data),
//   - sealed under a retired key (still in SECRETS_ENCRYPTION_PREVIOUS_KEYS), or
//   - not sealed at all: signing keys stored as a plaintext PEM, and tenant
//     password peppers from before `password_hash_secret_sealed` existed.
//
// Safe on every replica at once: each write is a compare-and-swap on the old value
// (`WHERE id = $1 AND <column> = $old`), so two replicas re-sealing the same row
// cannot overwrite each other, and a row already upgraded is skipped next time.
// A value that cannot be decrypted is left untouched and counted — see
// `repair_encrypted_secrets` for what can be done about those.
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::application::ports::secret_cipher::{SecretCipher, SecretContext, SecretPurpose};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResealReport {
    pub resealed: usize,
    pub unreadable: usize,
}

impl ResealReport {
    fn add(&mut self, other: ResealReport) {
        self.resealed += other.resealed;
        self.unreadable += other.unreadable;
    }
}

// One sealed column. All identifiers are static literals: no dynamic SQL is built
// from data.
struct SealedColumn {
    table: &'static str,
    column: &'static str,
    purpose: SecretPurpose,
}

const SEALED_COLUMNS: &[SealedColumn] = &[
    SealedColumn {
        table: "tenant_emails",
        column: "password_encrypted",
        purpose: SecretPurpose::SmtpPassword,
    },
    SealedColumn {
        table: "webhook_subscriptions",
        column: "secret_encrypted",
        purpose: SecretPurpose::WebhookSecret,
    },
    SealedColumn {
        table: "external_identity_providers",
        column: "provider_client_secret_encrypted",
        purpose: SecretPurpose::ExternalIdpClientSecret,
    },
    SealedColumn {
        table: "mfa_enrollments",
        column: "secret_enc",
        purpose: SecretPurpose::TotpSeed,
    },
    SealedColumn {
        table: "signing_keys",
        column: "private_key_encrypted",
        purpose: SecretPurpose::SigningKey,
    },
];

const PEM_MARKER: &str = "-----BEGIN";

pub async fn reseal_legacy_secrets(
    pool: &PgPool,
    cipher: &dyn SecretCipher,
) -> Result<ResealReport, sqlx::Error> {
    let mut report = ResealReport::default();
    for column in SEALED_COLUMNS {
        report.add(reseal_column(pool, cipher, column).await?);
    }
    report.add(reseal_tenant_peppers(pool, cipher).await?);
    Ok(report)
}

async fn reseal_column(
    pool: &PgPool,
    cipher: &dyn SecretCipher,
    column: &SealedColumn,
) -> Result<ResealReport, sqlx::Error> {
    let select = format!(
        "SELECT id, tenant_id, {col} AS value FROM {table} WHERE {col} IS NOT NULL",
        col = column.column,
        table = column.table
    );
    let update = format!(
        "UPDATE {table} SET {col} = $3 WHERE id = $1 AND {col} = $2",
        col = column.column,
        table = column.table
    );

    let mut report = ResealReport::default();
    // Safe: `table` / `column` come only from the static `SEALED_COLUMNS` table.
    for row in sqlx::query(sqlx::AssertSqlSafe(select))
        .fetch_all(pool)
        .await?
    {
        let value: String = row.get("value");
        if cipher.is_current(&value) {
            continue;
        }
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let context = SecretContext::new(column.purpose, tenant_id);

        // A signing key written before sealing existed is a plaintext PEM.
        let plaintext =
            if column.purpose == SecretPurpose::SigningKey && value.starts_with(PEM_MARKER) {
                Ok(value.clone())
            } else {
                cipher.decrypt(&value, context)
            };

        match plaintext.and_then(|p| cipher.encrypt(&p, context)) {
            Ok(sealed) => {
                sqlx::query(sqlx::AssertSqlSafe(update.as_str()))
                    .bind(id)
                    .bind(&value)
                    .bind(sealed)
                    .execute(pool)
                    .await?;
                report.resealed += 1;
            }
            Err(_) => {
                tracing::error!(
                    table = column.table,
                    %id,
                    "secret cannot be decrypted with any configured key; left unchanged \
                     (run `repair_encrypted_secrets`)"
                );
                report.unreadable += 1;
            }
        }
    }
    Ok(report)
}

async fn reseal_tenant_peppers(
    pool: &PgPool,
    cipher: &dyn SecretCipher,
) -> Result<ResealReport, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, password_hash_secret AS value, password_hash_secret_sealed AS sealed \
         FROM tenants",
    )
    .fetch_all(pool)
    .await?;

    let mut report = ResealReport::default();
    for row in rows {
        let id: Uuid = row.get("id");
        let value: String = row.get("value");
        let sealed: bool = row.get("sealed");
        if sealed && cipher.is_current(&value) {
            continue;
        }
        let context = SecretContext::new(SecretPurpose::TenantPepper, id);
        let plaintext = if sealed {
            cipher.decrypt(&value, context)
        } else {
            Ok(value.clone())
        };
        match plaintext.and_then(|p| cipher.encrypt(&p, context)) {
            Ok(new_value) => {
                sqlx::query(
                    "UPDATE tenants SET password_hash_secret = $3, \
                     password_hash_secret_sealed = TRUE \
                     WHERE id = $1 AND password_hash_secret = $2",
                )
                .bind(id)
                .bind(&value)
                .bind(new_value)
                .execute(pool)
                .await?;
                report.resealed += 1;
            }
            Err(_) => {
                tracing::error!(tenant_id = %id, "tenant pepper cannot be decrypted; left unchanged");
                report.unreadable += 1;
            }
        }
    }
    Ok(report)
}
