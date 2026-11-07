// Schema migrations, and the adoption of databases created before the squash.
//
// On 2026-11-10 the 71 migrations written since 2026-05-30 were squashed into one
// baseline, `migrations/20261110000000_initial_schema.sql`, which creates the schema
// they produced in one step (verified identical with pg_dump). A fresh database just
// runs it.
//
// A database created before the squash still lists the 71 old versions in
// `_sqlx_migrations`, and the migrator refuses a database whose applied migrations
// are missing from the source. If that database is up to date with the old
// history, its schema already IS the baseline, so its history is rewritten to say
// so: the old rows are replaced by the baseline's (Flyway calls this "baseline on
// migrate"). Only a complete history is adopted. Anything older stops the boot
// rather than guessing at a partial schema.
use sqlx::{PgPool, Row, migrate::Migrator};

/// Every migration of this crate (embedded at compile time).
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// The squashed baseline.
pub const BASELINE_VERSION: i64 = 20261110000000;

/// The migrations the baseline replaced, oldest first.
pub const PRE_SQUASH_VERSIONS: &[i64] = &[
    20260530145112,
    20260530160033,
    20260530164020,
    20260530164024,
    20260602005303,
    20260602005347,
    20260602005458,
    20260602005600,
    20260602005658,
    20260602005700,
    20260603120000,
    20260603130000,
    20260604120000,
    20260604130000,
    20260604140000,
    20260606011112,
    20260606011113,
    20260606021659,
    20260606021700,
    20260607000000,
    20260607000001,
    20260607000002,
    20260608000000,
    20260609000000,
    20260609000001,
    20260609000002,
    20260609000003,
    20260609000004,
    20260609000005,
    20260609000006,
    20260610000000,
    20260610000001,
    20260610000002,
    20260610000003,
    20260612000000,
    20260616000000,
    20260617000000,
    20260617001000,
    20260618000000,
    20260618000001,
    20260619000000,
    20260619000100,
    20260619000200,
    20260620000000,
    20260622000000,
    20260622000100,
    20260701000000,
    20260701000100,
    20260701000200,
    20260701000300,
    20260701000400,
    20260714000100,
    20260714000200,
    20260716000100,
    20260722000000,
    20260723000000,
    20260723000100,
    20260724000100,
    20260724000200,
    20260724000300,
    20260806000000,
    20260806220000,
    20260807000000,
    20260808000000,
    20261104000000,
    20261104000001,
    20261105000000,
    20261106000000,
    20261107000000,
    20261108000000,
    20261109000000,
];

// The last pre-squash migration was added shortly before the squash, so a database
// can be complete up to the one before it. That one step is replayed here, verbatim,
// before adopting.
const CATCH_UP_VERSION: i64 = 20261109000000;
const CATCH_UP_SQL: &str = r#"
-- Browser sessions of the platform-admin console, used when this server serves the
-- dashboard itself (UI_DIR). See docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md.
--
-- The browser holds a random 256-bit id in an httpOnly cookie; only its SHA-256 is
-- stored here, so reading this table yields nothing a browser could present. The
-- row holds NO token either: the `/api` cookie bridge mints a short-lived admin
-- access token per request, after re-checking that the admin is still active.
CREATE TABLE admin_web_sessions (
  id UUID PRIMARY KEY,
  admin_id UUID NOT NULL REFERENCES admin_users (id) ON DELETE CASCADE,

  token_hash VARCHAR(64) NOT NULL,

  created_at TIMESTAMPTZ NOT NULL,
  -- Idle timeout: a session unused for too long stops working before `expires_at`.
  last_seen_at TIMESTAMPTZ NOT NULL,
  -- Absolute lifetime, fixed at sign-in.
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX uq_admin_web_sessions_token_hash ON admin_web_sessions (token_hash);
CREATE INDEX ix_admin_web_sessions_admin ON admin_web_sessions (admin_id);
CREATE INDEX ix_admin_web_sessions_expires ON admin_web_sessions (expires_at);
"#;

/// What [`adopt_pre_squash_history`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Adoption {
    /// Fresh database, or one already on the baseline: nothing to do.
    NotNeeded,
    /// The old history was replaced by the baseline.
    Adopted {
        /// Old migration rows replaced.
        replaced: usize,
        /// Whether the last pre-squash migration had to be replayed first.
        caught_up: bool,
    },
}

/// Rewrites a pre-squash migration history to the baseline. Run it before
/// [`MIGRATOR`]`.run`, with the same (schema-owner) connection.
pub async fn adopt_pre_squash_history(pool: &PgPool) -> Result<Adoption, String> {
    let fail = |e: sqlx::Error| format!("adopting the squashed migration baseline: {e}");

    let has_history: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(fail)?;
    if !has_history {
        return Ok(Adoption::NotNeeded);
    }

    let mut tx = pool.begin().await.map_err(fail)?;
    // Several replicas may boot at once: one adopts, the others then see the result.
    sqlx::query("LOCK TABLE _sqlx_migrations IN EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await
        .map_err(fail)?;

    let rows = sqlx::query("SELECT version, success FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&mut *tx)
        .await
        .map_err(fail)?;
    let applied: Vec<(i64, bool)> = rows
        .iter()
        .map(|row| (row.get::<i64, _>("version"), row.get::<bool, _>("success")))
        .collect();

    let old: Vec<i64> = applied
        .iter()
        .map(|(version, _)| *version)
        .filter(|version| PRE_SQUASH_VERSIONS.contains(version))
        .collect();
    if old.is_empty() {
        return Ok(Adoption::NotNeeded);
    }
    if let Some((version, _)) = applied.iter().find(|(_, success)| !success) {
        return Err(format!(
            "migration {version} is recorded as failed; repair the database before upgrading"
        ));
    }

    let missing: Vec<i64> = PRE_SQUASH_VERSIONS
        .iter()
        .copied()
        .filter(|version| !old.contains(version))
        .collect();
    let caught_up = match missing.as_slice() {
        [] => false,
        [only] if *only == CATCH_UP_VERSION => {
            sqlx::raw_sql(CATCH_UP_SQL)
                .execute(&mut *tx)
                .await
                .map_err(fail)?;
            true
        }
        _ => {
            return Err(format!(
                "this database predates the migration squash and is missing {} of its \
                 migrations (first: {}). Upgrade it with a release from before the squash \
                 (any commit before 20261110000000_initial_schema.sql) first, then with this one.",
                missing.len(),
                missing[0]
            ));
        }
    };

    let baseline = MIGRATOR
        .iter()
        .find(|m| m.version == BASELINE_VERSION)
        .expect("the baseline migration is embedded");
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ANY($1)")
        .bind(PRE_SQUASH_VERSIONS)
        .execute(&mut *tx)
        .await
        .map_err(fail)?;
    // The row the migrator itself writes after applying a migration; `execution_time`
    // -1 marks it as recorded, not run.
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES ($1, $2, TRUE, $3, -1)",
    )
    .bind(baseline.version)
    .bind(baseline.description.as_ref())
    .bind(baseline.checksum.as_ref())
    .execute(&mut *tx)
    .await
    .map_err(fail)?;
    tx.commit().await.map_err(fail)?;

    Ok(Adoption::Adopted {
        replaced: old.len(),
        caught_up,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_baseline_comes_first_and_follows_the_old_history() {
        let first = MIGRATOR.iter().map(|m| m.version).min();
        assert_eq!(first, Some(BASELINE_VERSION));
        assert_eq!(PRE_SQUASH_VERSIONS.len(), 71);
        assert!(PRE_SQUASH_VERSIONS.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(PRE_SQUASH_VERSIONS.last(), Some(&CATCH_UP_VERSION));
        assert!(CATCH_UP_VERSION < BASELINE_VERSION);
    }

    #[test]
    fn the_baseline_cannot_be_reverted() {
        let baseline: Vec<_> = MIGRATOR
            .iter()
            .filter(|m| m.version == BASELINE_VERSION)
            .collect();
        assert_eq!(baseline.len(), 1, "the baseline has no down migration");
        assert_eq!(
            baseline[0].migration_type,
            sqlx::migrate::MigrationType::Simple
        );
    }
}
