// In-process caches for data that is read constantly and changes almost never.
//
// Deliberately per-process and derived-only: nothing here is authoritative, so a
// replica warming its own copy is fine and no shared store is required. Anything
// that must be consistent across replicas belongs in Postgres instead (the risk
// engine's velocity counters are computed that way for exactly this reason).
pub mod caching_risk_model_repository;
pub mod caching_signing_key_repository;
