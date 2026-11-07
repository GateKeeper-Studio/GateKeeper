// Fraud / suspicious-login detection admin API. Read use cases take repository arcs
// directly; write use cases (block/unblock IP, update policy) take the UnitOfWork
// factory. Serialize DTOs live in `views`.
pub mod views;

pub mod block_ip;
pub mod get_login_event;
pub mod get_overview;
pub mod get_risk_policy;
pub mod label_login_event;
pub mod list_attacked_users;
pub mod list_login_events;
pub mod list_suspicious_ips;
pub mod unblock_ip;
pub mod update_risk_policy;
