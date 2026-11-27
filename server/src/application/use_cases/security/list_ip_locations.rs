use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::{
        ports::geoip_resolver::GeoIpResolver,
        use_cases::security::views::{GeoAttributionView, IpLocationView},
    },
    domain::{
        entities::security_block::BlockType,
        errors::DomainError,
        repositories::{
            login_event_repository::LoginEventRepository,
            security_block_repository::SecurityBlockRepository,
        },
    },
};

// The points of the security maps: where a tenant's sign-in attempts come from, one
// point per IP, riskiest first, each with its current block status. `user_id` narrows
// it to one user (the user page's sign-in map). Read-only.
pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct Response {
    pub items: Vec<IpLocationView>,
    // Attempts in the window that had no coordinates (private addresses, or no GeoIP
    // database configured), so the map can say what it leaves out.
    pub unlocated_attempts: i64,
    // The credit the loaded geolocation database's licence asks for, if any.
    pub attribution: Option<GeoAttributionView>,
}

pub const DEFAULT_WINDOW_DAYS: i64 = 30;
pub const DEFAULT_LIMIT: i64 = 200;
pub const MAX_LIMIT: i64 = 500;

pub struct UseCase {
    login_event_repository: Arc<dyn LoginEventRepository>,
    security_block_repository: Arc<dyn SecurityBlockRepository>,
    // Only for the licence credit, read per request: the database can be swapped
    // while the server runs.
    geoip: Arc<dyn GeoIpResolver>,
}

impl UseCase {
    pub fn new(
        login_event_repository: Arc<dyn LoginEventRepository>,
        security_block_repository: Arc<dyn SecurityBlockRepository>,
        geoip: Arc<dyn GeoIpResolver>,
    ) -> Self {
        Self {
            login_event_repository,
            security_block_repository,
            geoip,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let to = query.to.unwrap_or_else(Utc::now);
        let from = query
            .from
            .unwrap_or_else(|| to - Duration::days(DEFAULT_WINDOW_DAYS));
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

        let locations = self
            .login_event_repository
            .ip_locations(query.tenant_id, query.user_id, from, to, limit)
            .await?;
        let unlocated_attempts = self
            .login_event_repository
            .count_unlocated(query.tenant_id, query.user_id, from, to)
            .await?;

        // One read for every active block, rather than one per point.
        let blocks = self
            .security_block_repository
            .list_active(query.tenant_id, Utc::now())
            .await?;
        let ip_blocks: HashMap<&str, _> = blocks
            .iter()
            .filter(|b| b.block_type() == BlockType::Ip)
            .filter_map(|b| b.ip_address().map(|ip| (ip, b)))
            .collect();

        let items = locations
            .into_iter()
            .map(|location| {
                let block = ip_blocks.get(location.ip_address.as_str()).copied();
                IpLocationView::from_parts(location, block)
            })
            .collect();

        Ok(Response {
            items,
            unlocated_attempts,
            attribution: self.geoip.attribution().map(GeoAttributionView::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::ports::geoip_resolver::GeoIpAttribution,
        domain::{
            entities::{
                login_event::{LoginEvent, RiskDecision, RiskLevel},
                security_block::SecurityBlock,
            },
            repositories::security_block_repository::SecurityBlockRepository,
        },
        testing::{
            fake_geoip_resolver::FakeGeoIpResolver,
            in_memory_repositories::{
                in_memory_login_event_repository::InMemoryLoginEventRepository,
                in_memory_security_block_repository::InMemorySecurityBlockRepository,
            },
        },
    };

    struct Fx {
        use_case: UseCase,
        events: Arc<InMemoryLoginEventRepository>,
        blocks: Arc<InMemorySecurityBlockRepository>,
        tenant_id: Uuid,
    }

    fn fixture() -> Fx {
        let events = Arc::new(InMemoryLoginEventRepository::new());
        let blocks = Arc::new(InMemorySecurityBlockRepository::new());
        Fx {
            use_case: UseCase::new(
                events.clone(),
                blocks.clone(),
                Arc::new(FakeGeoIpResolver::new()),
            ),
            events,
            blocks,
            tenant_id: Uuid::now_v7(),
        }
    }

    // A sign-in attempt from `ip`, placed at `place` (None: not geolocated).
    fn attempt(
        tenant_id: Uuid,
        user_id: Uuid,
        ip: &str,
        place: Option<(&str, f64, f64)>,
        success: bool,
    ) -> LoginEvent {
        let event = LoginEvent::new(tenant_id, Some(user_id), None, success, success).with_network(
            Some(ip.to_string()),
            None,
            None,
            None,
        );
        match place {
            Some((city, lat, lon)) => event.with_geo(
                Some("BR".to_string()),
                Some("SP".to_string()),
                Some(city.to_string()),
                Some(lat),
                Some(lon),
                Some(64512),
                Some("Example ISP".to_string()),
            ),
            None => event,
        }
    }

    fn query(fx: &Fx, user_id: Option<Uuid>) -> Query {
        Query {
            tenant_id: fx.tenant_id,
            user_id,
            from: None,
            to: None,
            limit: None,
        }
    }

    #[tokio::test]
    async fn groups_attempts_by_ip_at_the_latest_location() {
        let fx = fixture();
        let user = Uuid::now_v7();
        let ip = "203.0.113.7";
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    user,
                    ip,
                    Some(("Campinas", -22.9, -47.06)),
                    false,
                ),
                &[],
            )
            .await
            .unwrap();
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    user,
                    ip,
                    Some(("São Paulo", -23.55, -46.63)),
                    true,
                ),
                &[],
            )
            .await
            .unwrap();

        let response = fx.use_case.execute(query(&fx, None)).await.unwrap();

        assert_eq!(response.items.len(), 1);
        let point = &response.items[0];
        assert_eq!(point.ip_address, ip);
        assert_eq!(point.city.as_deref(), Some("São Paulo"));
        // City-level only: rounded to 0.1°.
        assert_eq!((point.latitude, point.longitude), (-23.6, -46.6));
        assert_eq!(point.attempts, 2);
        assert_eq!(point.successful_logins, 1);
        assert_eq!(point.failed_attempts, 1);
        assert_eq!(point.distinct_users, 1);
        assert_eq!(point.isp.as_deref(), Some("Example ISP"));
        assert!(!point.blocked);
    }

    #[tokio::test]
    async fn attempts_without_coordinates_are_counted_not_placed() {
        let fx = fixture();
        let user = Uuid::now_v7();
        fx.events
            .add(&attempt(fx.tenant_id, user, "10.0.0.5", None, true), &[])
            .await
            .unwrap();
        fx.events
            .add(&attempt(fx.tenant_id, user, "127.0.0.1", None, false), &[])
            .await
            .unwrap();

        let response = fx.use_case.execute(query(&fx, None)).await.unwrap();

        assert!(response.items.is_empty());
        assert_eq!(response.unlocated_attempts, 2);
    }

    #[tokio::test]
    async fn the_riskiest_ip_comes_first() {
        let fx = fixture();
        let user = Uuid::now_v7();
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    user,
                    "198.51.100.1",
                    Some(("Lisbon", 38.7, -9.1)),
                    true,
                ),
                &[],
            )
            .await
            .unwrap();
        let attack = attempt(
            fx.tenant_id,
            user,
            "198.51.100.66",
            Some(("Moscow", 55.75, 37.6)),
            false,
        )
        .with_risk(95, RiskLevel::Critical, RiskDecision::BlockTemporarily);
        fx.events.add(&attack, &[]).await.unwrap();

        let response = fx.use_case.execute(query(&fx, None)).await.unwrap();

        assert_eq!(response.items.len(), 2);
        let first = &response.items[0];
        assert_eq!(first.ip_address, "198.51.100.66");
        assert_eq!(first.max_risk_level, "CRITICAL");
        assert_eq!(first.max_risk_score, 95);
        assert_eq!(first.blocked_attempts, 1);
        assert_eq!(response.items[1].max_risk_level, "LOW");
    }

    #[tokio::test]
    async fn a_user_filter_keeps_only_that_users_sign_ins() {
        let fx = fixture();
        let alice = Uuid::now_v7();
        let bob = Uuid::now_v7();
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    alice,
                    "192.0.2.10",
                    Some(("Recife", -8.05, -34.9)),
                    true,
                ),
                &[],
            )
            .await
            .unwrap();
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    bob,
                    "192.0.2.20",
                    Some(("Porto Alegre", -30.03, -51.2)),
                    true,
                ),
                &[],
            )
            .await
            .unwrap();
        fx.events
            .add(&attempt(fx.tenant_id, bob, "10.1.1.1", None, true), &[])
            .await
            .unwrap();

        let response = fx.use_case.execute(query(&fx, Some(alice))).await.unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].ip_address, "192.0.2.10");
        // Bob's unlocated attempt is not Alice's.
        assert_eq!(response.unlocated_attempts, 0);
    }

    #[tokio::test]
    async fn another_tenants_attempts_never_show() {
        let fx = fixture();
        fx.events
            .add(
                &attempt(
                    Uuid::now_v7(),
                    Uuid::now_v7(),
                    "192.0.2.30",
                    Some(("Lima", -12.05, -77.04)),
                    true,
                ),
                &[],
            )
            .await
            .unwrap();

        let response = fx.use_case.execute(query(&fx, None)).await.unwrap();

        assert!(response.items.is_empty());
        assert_eq!(response.unlocated_attempts, 0);
    }

    #[tokio::test]
    async fn an_active_ip_block_marks_its_point() {
        let fx = fixture();
        let ip = "203.0.113.99";
        fx.events
            .add(
                &attempt(
                    fx.tenant_id,
                    Uuid::now_v7(),
                    ip,
                    Some(("Bogotá", 4.7, -74.07)),
                    false,
                ),
                &[],
            )
            .await
            .unwrap();
        let block = SecurityBlock::new_ip_block(
            fx.tenant_id,
            ip.to_string(),
            "manual".to_string(),
            "admin".to_string(),
            Utc::now() + Duration::hours(1),
            None,
        );
        fx.blocks.add(&block).await.unwrap();
        // An expired block on the same IP changes nothing.
        let expired = SecurityBlock::new_ip_block(
            fx.tenant_id,
            ip.to_string(),
            "old".to_string(),
            "admin".to_string(),
            Utc::now() - Duration::hours(1),
            None,
        );
        fx.blocks.add(&expired).await.unwrap();

        let response = fx.use_case.execute(query(&fx, None)).await.unwrap();

        let point = &response.items[0];
        assert!(point.blocked);
        assert_eq!(point.block_id, Some(block.id().to_string()));
        assert!(point.block_expires_at.is_some());
    }

    #[tokio::test]
    async fn the_limit_caps_the_points() {
        let fx = fixture();
        for i in 1..=3 {
            fx.events
                .add(
                    &attempt(
                        fx.tenant_id,
                        Uuid::now_v7(),
                        &format!("192.0.2.{i}"),
                        Some(("Quito", -0.18, -78.47)),
                        true,
                    ),
                    &[],
                )
                .await
                .unwrap();
        }

        let response = fx
            .use_case
            .execute(Query {
                limit: Some(2),
                ..query(&fx, None)
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 2);
    }

    #[tokio::test]
    async fn the_database_credit_travels_with_the_points() {
        let events = Arc::new(InMemoryLoginEventRepository::new());
        let use_case = UseCase::new(
            events,
            Arc::new(InMemorySecurityBlockRepository::new()),
            Arc::new(FakeGeoIpResolver::new().with_attribution(GeoIpAttribution {
                name: "IP geolocation by DB-IP",
                url: "https://db-ip.com",
            })),
        );

        let response = use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
                user_id: None,
                from: None,
                to: None,
                limit: None,
            })
            .await
            .unwrap();

        let attribution = response.attribution.unwrap();
        assert_eq!(attribution.name, "IP geolocation by DB-IP");
        assert_eq!(attribution.url, "https://db-ip.com");
    }
}
