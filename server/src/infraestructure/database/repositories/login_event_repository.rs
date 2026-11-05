use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::login_event::{
            LoginEvent, RiskDecision, RiskLevel, RiskSeverity, RiskSignal, RiskSignalType,
        },
        errors::DomainError,
        repositories::login_event_repository::{
            AttackedUser, CountryCount, LastSuccessfulLogin, LoginEventFilter,
            LoginEventRepository, SecurityOverviewMetrics, SuspiciousIp,
        },
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbLoginEventRepository {
    executor: DbExecutor,
}

impl DbLoginEventRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

// The full-row projection shared by every "return LoginEvent(s)" query.
struct LoginEventRow {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    client_id: Option<Uuid>,
    username_normalized: Option<String>,
    username_hash: Option<String>,
    ip_address: Option<String>,
    ip_hash: Option<String>,
    user_agent: Option<String>,
    user_agent_hash: Option<String>,
    country_code: Option<String>,
    region: Option<String>,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    asn: Option<i32>,
    isp: Option<String>,
    credentials_valid: bool,
    success: bool,
    failure_reason: Option<String>,
    risk_score: i32,
    risk_level: String,
    decision: String,
    created_at: DateTime<Utc>,
}

impl LoginEventRow {
    fn into_entity(self) -> LoginEvent {
        LoginEvent::reconstruct(
            self.id,
            self.tenant_id,
            self.user_id,
            self.client_id,
            self.username_normalized,
            self.username_hash,
            self.ip_address,
            self.ip_hash,
            self.user_agent,
            self.user_agent_hash,
            self.country_code,
            self.region,
            self.city,
            self.latitude,
            self.longitude,
            self.asn,
            self.isp,
            self.credentials_valid,
            self.success,
            self.failure_reason,
            self.risk_score,
            RiskLevel::from_str(&self.risk_level),
            RiskDecision::from_str(&self.decision),
            self.created_at,
        )
    }
}

#[async_trait]
impl LoginEventRepository for DbLoginEventRepository {
    async fn add(&self, event: &LoginEvent, signals: &[RiskSignal]) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          INSERT INTO "login_events"
            (id, tenant_id, user_id, client_id, username_normalized, username_hash,
             ip_address, ip_hash, user_agent, user_agent_hash, country_code, region,
             city, latitude, longitude, asn, isp, credentials_valid, success,
             failure_reason, risk_score, risk_level, decision, created_at)
          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                  $16, $17, $18, $19, $20, $21, $22, $23, $24)
        "#,
                event.id(),
                event.tenant_id(),
                event.user_id(),
                event.client_id(),
                event.username_normalized(),
                event.username_hash(),
                event.ip_address(),
                event.ip_hash(),
                event.user_agent(),
                event.user_agent_hash(),
                event.country_code(),
                event.region(),
                event.city(),
                event.latitude(),
                event.longitude(),
                event.asn(),
                event.isp(),
                event.credentials_valid(),
                event.success(),
                event.failure_reason(),
                event.risk_score(),
                event.risk_level().as_str(),
                event.decision().as_str(),
                event.created_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        for signal in signals {
            run!(
                self.executor,
                sqlx::query!(
                    r#"
              INSERT INTO "login_risk_signals"
                (id, login_event_id, tenant_id, signal_type, severity, points,
                 metadata_json, created_at)
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
                    Uuid::now_v7(),
                    event.id(),
                    event.tenant_id(),
                    signal.signal_type().as_str(),
                    signal.severity().as_str(),
                    signal.points(),
                    signal.metadata(),
                    Utc::now(),
                ),
                execute
            )
            .map_err(map_sqlx_error)?;
        }

        Ok(())
    }

    async fn count_failed_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) FROM "login_events"
                   WHERE tenant_id = $1 AND ip_address = $2
                     AND credentials_valid = false AND created_at >= $3"#,
                tenant_id,
                ip,
                since
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count.unwrap_or(0))
    }

    async fn count_failed_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) FROM "login_events"
                   WHERE tenant_id = $1 AND user_id = $2
                     AND credentials_valid = false AND created_at >= $3"#,
                tenant_id,
                user_id,
                since
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count.unwrap_or(0))
    }

    async fn count_distinct_users_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT COUNT(DISTINCT COALESCE(username_normalized, user_id::text))
                   FROM "login_events"
                   WHERE tenant_id = $1 AND ip_address = $2 AND created_at >= $3"#,
                tenant_id,
                ip,
                since
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count.unwrap_or(0))
    }

    async fn count_tenant_failures(
        &self,
        tenant_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<i64, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT COUNT(*) FROM "login_events"
                   WHERE tenant_id = $1 AND credentials_valid = false AND created_at >= $2"#,
                tenant_id,
                since
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count.unwrap_or(0))
    }

    async fn last_successful_login(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LastSuccessfulLogin>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT latitude, longitude, country_code, created_at
                   FROM "login_events"
                   WHERE tenant_id = $1 AND user_id = $2 AND success = true
                   ORDER BY created_at DESC
                   LIMIT 1"#,
                tenant_id,
                user_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| LastSuccessfulLogin {
            latitude: r.latitude,
            longitude: r.longitude,
            country_code: r.country_code,
            created_at: r.created_at,
        }))
    }

    async fn has_seen_country(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        country_code: &str,
    ) -> Result<bool, DomainError> {
        let seen = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT EXISTS(
                     SELECT 1 FROM "login_events"
                     WHERE tenant_id = $1 AND user_id = $2 AND success = true
                       AND country_code = $3
                   )"#,
                tenant_id,
                user_id,
                country_code
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(seen.unwrap_or(false))
    }

    async fn has_seen_user_agent(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        user_agent_hash: &str,
    ) -> Result<bool, DomainError> {
        let seen = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"SELECT EXISTS(
                     SELECT 1 FROM "login_events"
                     WHERE tenant_id = $1 AND user_id = $2 AND success = true
                       AND user_agent_hash = $3
                   )"#,
                tenant_id,
                user_id,
                user_agent_hash
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(seen.unwrap_or(false))
    }

    async fn list(
        &self,
        tenant_id: Uuid,
        filter: &LoginEventFilter,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query_as!(
                LoginEventRow,
                r#"
          SELECT id, tenant_id, user_id, client_id, username_normalized, username_hash,
                 ip_address, ip_hash, user_agent, user_agent_hash, country_code, region,
                 city, latitude, longitude, asn, isp, credentials_valid, success,
                 failure_reason, risk_score, risk_level, decision, created_at
          FROM "login_events"
          WHERE tenant_id = $1
            AND ($2::uuid IS NULL OR user_id = $2)
            AND ($3::text IS NULL OR ip_address = $3)
            AND ($4::text IS NULL OR country_code = $4)
            AND ($5::text IS NULL OR risk_level = $5)
            AND ($6::text IS NULL OR decision = $6)
            AND ($7::bool IS NULL OR success = $7)
            AND ($8::timestamptz IS NULL OR created_at >= $8)
            AND ($9::timestamptz IS NULL OR created_at <= $9)
            AND ($10::text IS NULL OR EXISTS (
                  SELECT 1 FROM "login_risk_signals" s
                  WHERE s.login_event_id = login_events.id AND s.signal_type = $10))
          ORDER BY created_at DESC
          LIMIT $11 OFFSET $12
        "#,
                tenant_id,
                filter.user_id,
                filter.ip_address,
                filter.country_code,
                filter.risk_level,
                filter.decision,
                filter.success,
                filter.from,
                filter.to,
                filter.signal_type,
                page_size as i64,
                ((page.max(1) - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(LoginEventRow::into_entity).collect())
    }

    async fn count(&self, tenant_id: Uuid, filter: &LoginEventFilter) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!(
                r#"
          SELECT COUNT(*)
          FROM "login_events"
          WHERE tenant_id = $1
            AND ($2::uuid IS NULL OR user_id = $2)
            AND ($3::text IS NULL OR ip_address = $3)
            AND ($4::text IS NULL OR country_code = $4)
            AND ($5::text IS NULL OR risk_level = $5)
            AND ($6::text IS NULL OR decision = $6)
            AND ($7::bool IS NULL OR success = $7)
            AND ($8::timestamptz IS NULL OR created_at >= $8)
            AND ($9::timestamptz IS NULL OR created_at <= $9)
            AND ($10::text IS NULL OR EXISTS (
                  SELECT 1 FROM "login_risk_signals" s
                  WHERE s.login_event_id = login_events.id AND s.signal_type = $10))
        "#,
                tenant_id,
                filter.user_id,
                filter.ip_address,
                filter.country_code,
                filter.risk_level,
                filter.decision,
                filter.success,
                filter.from,
                filter.to,
                filter.signal_type,
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;
        Ok(count.unwrap_or(0) as u32)
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<LoginEvent>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query_as!(
                LoginEventRow,
                r#"
          SELECT id, tenant_id, user_id, client_id, username_normalized, username_hash,
                 ip_address, ip_hash, user_agent, user_agent_hash, country_code, region,
                 city, latitude, longitude, asn, isp, credentials_valid, success,
                 failure_reason, risk_score, risk_level, decision, created_at
          FROM "login_events"
          WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(LoginEventRow::into_entity))
    }

    async fn signals_for(&self, login_event_id: Uuid) -> Result<Vec<RiskSignal>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT signal_type, severity, points, metadata_json
                   FROM "login_risk_signals"
                   WHERE login_event_id = $1
                   ORDER BY points DESC"#,
                login_event_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                RiskSignalType::from_str(&r.signal_type).map(|signal_type| {
                    RiskSignal::reconstruct(
                        signal_type,
                        RiskSeverity::from_str(&r.severity),
                        r.points,
                        r.metadata_json.unwrap_or_else(|| serde_json::json!({})),
                    )
                })
            })
            .collect())
    }

    async fn overview_metrics(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<SecurityOverviewMetrics, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT
            COUNT(*) AS total,
            COUNT(*) FILTER (WHERE success) AS successful,
            COUNT(*) FILTER (WHERE NOT credentials_valid) AS failed,
            COUNT(*) FILTER (WHERE decision = 'BLOCK_TEMPORARILY') AS blocked,
            COUNT(*) FILTER (WHERE risk_level = 'HIGH') AS high,
            COUNT(*) FILTER (WHERE risk_level = 'CRITICAL') AS critical,
            COUNT(*) FILTER (WHERE EXISTS (
              SELECT 1 FROM "login_risk_signals" s
              WHERE s.login_event_id = login_events.id
                AND s.signal_type = 'PASSWORD_SPRAYING')) AS spraying
          FROM "login_events"
          WHERE tenant_id = $1 AND created_at >= $2 AND created_at <= $3
        "#,
                tenant_id,
                from,
                to
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(SecurityOverviewMetrics {
            total_attempts: row.total.unwrap_or(0),
            successful_logins: row.successful.unwrap_or(0),
            failed_logins: row.failed.unwrap_or(0),
            blocked_attempts: row.blocked.unwrap_or(0),
            high_risk_attempts: row.high.unwrap_or(0),
            critical_risk_attempts: row.critical.unwrap_or(0),
            password_spraying_events: row.spraying.unwrap_or(0),
        })
    }

    async fn logins_by_country(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CountryCount>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT country_code, COUNT(*) AS cnt
          FROM "login_events"
          WHERE tenant_id = $1 AND created_at >= $2 AND created_at <= $3
          GROUP BY country_code
          ORDER BY cnt DESC
        "#,
                tenant_id,
                from,
                to
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| CountryCount {
                country_code: r.country_code,
                count: r.cnt.unwrap_or(0),
            })
            .collect())
    }

    async fn top_suspicious_ips(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<SuspiciousIp>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT
            ip_address AS "ip_address!",
            COUNT(*) AS "attempts!",
            COUNT(*) FILTER (WHERE NOT credentials_valid) AS "failed_attempts!",
            COUNT(DISTINCT COALESCE(username_normalized, user_id::text)) AS "distinct_users!",
            MAX(country_code) AS country_code,
            MAX(risk_score) AS "max_risk_score!",
            MIN(created_at) AS "first_seen!",
            MAX(created_at) AS "last_seen!"
          FROM "login_events"
          WHERE tenant_id = $1 AND ip_address IS NOT NULL
            AND created_at >= $2 AND created_at <= $3
          GROUP BY ip_address
          ORDER BY "failed_attempts!" DESC, "attempts!" DESC
          LIMIT $4
        "#,
                tenant_id,
                from,
                to,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| SuspiciousIp {
                ip_address: r.ip_address,
                attempts: r.attempts,
                failed_attempts: r.failed_attempts,
                distinct_users: r.distinct_users,
                country_code: r.country_code,
                max_risk_score: r.max_risk_score,
                first_seen: r.first_seen,
                last_seen: r.last_seen,
            })
            .collect())
    }

    async fn attacked_users(
        &self,
        tenant_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AttackedUser>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT
            user_id AS "user_id!",
            MAX(username_normalized) AS username,
            COUNT(*) FILTER (WHERE NOT credentials_valid) AS "failed_attempts!",
            COUNT(DISTINCT ip_address) AS "distinct_ips!",
            MAX(created_at) FILTER (WHERE NOT credentials_valid) AS last_failed_at,
            MAX(created_at) FILTER (WHERE success) AS last_success_at
          FROM "login_events"
          WHERE tenant_id = $1 AND user_id IS NOT NULL
            AND created_at >= $2 AND created_at <= $3
          GROUP BY user_id
          HAVING COUNT(*) FILTER (WHERE NOT credentials_valid) > 0
          ORDER BY "failed_attempts!" DESC
          LIMIT $4
        "#,
                tenant_id,
                from,
                to,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| AttackedUser {
                user_id: r.user_id,
                username: r.username,
                failed_attempts: r.failed_attempts,
                distinct_ips: r.distinct_ips,
                last_failed_at: r.last_failed_at,
                last_success_at: r.last_success_at,
            })
            .collect())
    }

    async fn related_by_ip(
        &self,
        tenant_id: Uuid,
        ip: &str,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query_as!(
                LoginEventRow,
                r#"
          SELECT id, tenant_id, user_id, client_id, username_normalized, username_hash,
                 ip_address, ip_hash, user_agent, user_agent_hash, country_code, region,
                 city, latitude, longitude, asn, isp, credentials_valid, success,
                 failure_reason, risk_score, risk_level, decision, created_at
          FROM "login_events"
          WHERE tenant_id = $1 AND ip_address = $2 AND id <> $3
          ORDER BY created_at DESC
          LIMIT $4
        "#,
                tenant_id,
                ip,
                exclude_id,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(LoginEventRow::into_entity).collect())
    }

    async fn related_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        exclude_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LoginEvent>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query_as!(
                LoginEventRow,
                r#"
          SELECT id, tenant_id, user_id, client_id, username_normalized, username_hash,
                 ip_address, ip_hash, user_agent, user_agent_hash, country_code, region,
                 city, latitude, longitude, asn, isp, credentials_valid, success,
                 failure_reason, risk_score, risk_level, decision, created_at
          FROM "login_events"
          WHERE tenant_id = $1 AND user_id = $2 AND id <> $3
          ORDER BY created_at DESC
          LIMIT $4
        "#,
                tenant_id,
                user_id,
                exclude_id,
                limit
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows.into_iter().map(LoginEventRow::into_entity).collect())
    }
}

pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("login_events database error: {e:?}");
    DomainError::InfrastructureError("Database error".to_string())
}
