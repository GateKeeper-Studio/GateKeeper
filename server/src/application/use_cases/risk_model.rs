// The statistical risk model's lifecycle (docs/RISK-SCORING.md): train a candidate
// from labeled login events, promote it (or roll back) by a deliberate operator step,
// and report how the serving model behaves. Platform-level and global — driven by
// the `train_risk_model` CLI, never by a tenant request.
pub mod promote;
pub mod report;
pub mod train;

#[cfg(test)]
mod tests;
