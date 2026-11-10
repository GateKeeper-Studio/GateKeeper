// The squashed migration baseline and the adoption of pre-squash databases
// (src/infraestructure/database/migrations.rs), against a real Postgres.
//
// Opt-in like tests/db_repositories.rs: `cargo test --features db-tests`.
#![cfg(feature = "db-tests")]

use sqlx::PgPool;
use web_api::infraestructure::database::migrations::{
    Adoption, BASELINE_VERSION, MIGRATOR, PRE_SQUASH_VERSIONS, adopt_pre_squash_history,
};

async fn history(pool: &PgPool) -> Vec<i64> {
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn table_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables
         WHERE table_schema = 'public' AND table_name <> '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

// A database as the old migrations left it: the final schema (built here from the
// baseline, which is identical) and the old history in `_sqlx_migrations`.
async fn pre_squash_database(pool: &PgPool, versions: &[i64]) {
    // Only the baseline: later migrations postdate the squash, so an old database
    // lacks them and gets them from the migrator after adoption.
    MIGRATOR.run_to(BASELINE_VERSION, pool).await.unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations")
        .execute(pool)
        .await
        .unwrap();
    for version in versions {
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
             VALUES ($1, 'pre-squash', TRUE, '\\x00', 1)",
        )
        .bind(version)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[sqlx::test(migrations = false)]
async fn a_fresh_database_is_created_by_the_baseline_alone(pool: PgPool) {
    assert_eq!(
        adopt_pre_squash_history(&pool).await,
        Ok(Adoption::NotNeeded)
    );
    MIGRATOR.run_to(BASELINE_VERSION, &pool).await.unwrap();
    assert_eq!(history(&pool).await, [BASELINE_VERSION]);
    assert_eq!(table_count(&pool).await, 42);

    // Every later migration applies on top (new tables: `admin_mfa`,
    // `admin_invitations`).
    MIGRATOR.run(&pool).await.unwrap();
    assert_eq!(history(&pool).await[0], BASELINE_VERSION);
    assert_eq!(table_count(&pool).await, 44);
    // The append-only audit trigger came with it.
    let triggers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_trigger WHERE tgname = 'audit_events_append_only'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(triggers, 1);
}

#[sqlx::test(migrations = false)]
async fn a_complete_pre_squash_history_is_adopted_and_then_migrates_cleanly(pool: PgPool) {
    pre_squash_database(&pool, PRE_SQUASH_VERSIONS).await;

    assert_eq!(
        adopt_pre_squash_history(&pool).await,
        Ok(Adoption::Adopted {
            replaced: 71,
            caught_up: false
        })
    );
    assert_eq!(history(&pool).await[0], BASELINE_VERSION);
    // The migrator accepts the adopted history (checksum included) and runs nothing.
    MIGRATOR.run(&pool).await.unwrap();
    assert_eq!(history(&pool).await[0], BASELINE_VERSION);
    // Adopting again is a no-op.
    assert_eq!(
        adopt_pre_squash_history(&pool).await,
        Ok(Adoption::NotNeeded)
    );
}

#[sqlx::test(migrations = false)]
async fn a_database_one_migration_behind_is_caught_up_then_adopted(pool: PgPool) {
    let (last, before) = PRE_SQUASH_VERSIONS.split_last().unwrap();
    pre_squash_database(&pool, before).await;
    // What that database lacks: the last pre-squash migration's table.
    sqlx::query("DROP TABLE admin_web_sessions")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        adopt_pre_squash_history(&pool).await,
        Ok(Adoption::Adopted {
            replaced: 70,
            caught_up: true
        })
    );
    assert!(*last < BASELINE_VERSION);
    assert_eq!(table_count(&pool).await, 42);
    MIGRATOR.run(&pool).await.unwrap();
    assert_eq!(history(&pool).await[0], BASELINE_VERSION);
}

#[sqlx::test(migrations = false)]
async fn an_older_pre_squash_database_is_refused_untouched(pool: PgPool) {
    pre_squash_database(&pool, &PRE_SQUASH_VERSIONS[..60]).await;

    let refusal = adopt_pre_squash_history(&pool).await.unwrap_err();
    assert!(
        refusal.contains("missing 11 of its migrations"),
        "{refusal}"
    );
    assert_eq!(history(&pool).await, PRE_SQUASH_VERSIONS[..60].to_vec());
}

#[sqlx::test(migrations = false)]
async fn a_failed_migration_row_is_refused(pool: PgPool) {
    pre_squash_database(&pool, PRE_SQUASH_VERSIONS).await;
    sqlx::query("UPDATE _sqlx_migrations SET success = FALSE WHERE version = $1")
        .bind(PRE_SQUASH_VERSIONS[10])
        .execute(&pool)
        .await
        .unwrap();

    let refusal = adopt_pre_squash_history(&pool).await.unwrap_err();
    assert!(refusal.contains("recorded as failed"), "{refusal}");
}
