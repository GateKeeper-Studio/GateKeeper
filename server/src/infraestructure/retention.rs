// Data-retention enforcement: the background job that deletes expired rows.
// See `docs/compliance/retention-schedule.md` for what each table's floor is.
pub mod scheduler;
