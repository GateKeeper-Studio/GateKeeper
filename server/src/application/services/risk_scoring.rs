// Statistical risk scoring (docs/RISK-SCORING.md): the feature vector the hybrid
// engine computes for each attempt, and the logistic-regression training that turns
// labeled vectors into a model. Pure code — no I/O — so all of it is unit-tested.
pub mod features;
pub mod training;
