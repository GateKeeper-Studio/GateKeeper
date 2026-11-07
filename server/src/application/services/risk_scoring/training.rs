// Logistic regression, trained from labeled feature vectors. Small and dependency-
// free on purpose: the model is a handful of weights a person can read, the fit is
// deterministic (same data, same model), and every step is unit-tested.
use crate::domain::entities::risk_model::{ModelParams, sigmoid};

// One labeled vector. `fraudulent` is the positive class.
#[derive(Debug, Clone)]
pub struct Sample {
    pub values: Vec<f64>,
    pub fraudulent: bool,
    // Admin labels weigh 1.0, weak MFA-outcome labels 0.3 (LabelSource).
    pub weight: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct FitOptions {
    // L2 penalty on the weights (not the intercept). Keeps the model from leaning on
    // any one feature harder than the data justifies.
    pub l2: f64,
    pub iterations: usize,
    pub learning_rate: f64,
}

impl Default for FitOptions {
    fn default() -> Self {
        FitOptions {
            l2: 0.01,
            iterations: 3000,
            learning_rate: 0.5,
        }
    }
}

// Quality of a model on held-out data, at its threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub auc: f64,
    pub threshold: f64,
    // Share of legitimate logins the model would send to step-up MFA.
    pub false_positive_rate: f64,
    pub precision: f64,
    pub recall: f64,
    pub legitimate: usize,
    pub fraudulent: usize,
}

// Fits a standardized logistic regression. The threshold is left at 0.5; pick the
// real one on validation data with `choose_threshold`.
//
// Classes are balanced: fraud is rare, and without balancing the cheapest fit would
// be "everything is legitimate". Sample weights multiply on top.
pub fn fit(samples: &[Sample], feature_names: &[String], options: FitOptions) -> ModelParams {
    let n_features = feature_names.len();
    let (means, stds) = standardization(samples, n_features);

    let total: f64 = samples.iter().map(|s| s.weight).sum();
    let class_total = |fraud: bool| -> f64 {
        samples
            .iter()
            .filter(|s| s.fraudulent == fraud)
            .map(|s| s.weight)
            .sum()
    };
    let (fraud_total, legit_total) = (class_total(true), class_total(false));
    let balance = |fraud: bool| -> f64 {
        let class = if fraud { fraud_total } else { legit_total };
        if class > 0.0 {
            total / (2.0 * class)
        } else {
            0.0
        }
    };

    let rows: Vec<(Vec<f64>, f64, f64)> = samples
        .iter()
        .map(|s| {
            let z = standardize(&s.values, &means, &stds);
            let y = if s.fraudulent { 1.0 } else { 0.0 };
            (z, y, s.weight * balance(s.fraudulent))
        })
        .collect();
    let weight_sum: f64 = rows
        .iter()
        .map(|(_, _, w)| w)
        .sum::<f64>()
        .max(f64::EPSILON);

    let mut weights = vec![0.0; n_features];
    let mut intercept = 0.0;
    for _ in 0..options.iterations {
        let mut grad = vec![0.0; n_features];
        let mut grad_intercept = 0.0;
        for (z, y, w) in &rows {
            let logit = intercept + dot(&weights, z);
            let error = (sigmoid(logit) - y) * w;
            for (g, zj) in grad.iter_mut().zip(z) {
                *g += error * zj;
            }
            grad_intercept += error;
        }
        for (wj, gj) in weights.iter_mut().zip(&grad) {
            *wj -= options.learning_rate * (gj / weight_sum + options.l2 * *wj);
        }
        intercept -= options.learning_rate * grad_intercept / weight_sum;
    }

    ModelParams {
        feature_names: feature_names.to_vec(),
        means,
        stds,
        weights,
        intercept,
        threshold: 0.5,
    }
}

// The lowest probability threshold whose false-positive rate on the legitimate
// samples stays at or below `target_fpr` — "safe against excessive false positives"
// made a number. Never below 0.5: below that the model itself thinks the login is
// more likely legitimate than not, and must not ask for MFA.
pub fn choose_threshold(params: &ModelParams, samples: &[Sample], target_fpr: f64) -> f64 {
    let mut legit: Vec<f64> = samples
        .iter()
        .filter(|s| !s.fraudulent)
        .filter_map(|s| params.score(&s.values).map(|x| x.probability))
        .collect();
    if legit.is_empty() {
        return 1.0;
    }
    legit.sort_by(|a, b| b.total_cmp(a));
    // At most `allowed` legitimate samples may reach the threshold.
    let allowed = (target_fpr.clamp(0.0, 1.0) * legit.len() as f64).floor() as usize;
    let threshold = if allowed >= legit.len() {
        0.0
    } else {
        // Just above the first legitimate probability that must NOT be flagged.
        next_up(legit[allowed])
    };
    threshold.clamp(0.5, 1.0)
}

pub fn evaluate(params: &ModelParams, samples: &[Sample]) -> Metrics {
    let scored: Vec<(f64, bool)> = samples
        .iter()
        .filter_map(|s| {
            params
                .score(&s.values)
                .map(|x| (x.probability, s.fraudulent))
        })
        .collect();
    let flagged = |s: &&(f64, bool)| s.0 >= params.threshold;

    let fraudulent = scored.iter().filter(|(_, f)| *f).count();
    let legitimate = scored.len() - fraudulent;
    let true_positives = scored.iter().filter(|s| s.1).filter(flagged).count();
    let false_positives = scored.iter().filter(|s| !s.1).filter(flagged).count();

    let ratio = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    Metrics {
        auc: auc(&scored),
        threshold: params.threshold,
        false_positive_rate: ratio(false_positives, legitimate),
        precision: ratio(true_positives, true_positives + false_positives),
        recall: ratio(true_positives, fraudulent),
        legitimate,
        fraudulent,
    }
}

// Area under the ROC curve (Mann–Whitney): the chance a random fraudulent sample
// scores above a random legitimate one. 0.5 is a coin; ties count half.
pub fn auc(scored: &[(f64, bool)]) -> f64 {
    let mut sorted: Vec<(f64, bool)> = scored.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let positives = sorted.iter().filter(|(_, f)| *f).count() as f64;
    let negatives = sorted.len() as f64 - positives;
    if positives == 0.0 || negatives == 0.0 {
        return 0.5;
    }

    // Sum of the positives' (1-based, tie-averaged) ranks.
    let mut rank_sum = 0.0;
    let mut i = 0;
    while i < sorted.len() {
        let mut j = i;
        while j + 1 < sorted.len() && sorted[j + 1].0 == sorted[i].0 {
            j += 1;
        }
        let average_rank = (i + j) as f64 / 2.0 + 1.0;
        rank_sum += sorted[i..=j].iter().filter(|(_, f)| *f).count() as f64 * average_rank;
        i = j + 1;
    }
    (rank_sum - positives * (positives + 1.0) / 2.0) / (positives * negatives)
}

// Means and standard deviations from the training samples. A constant feature gets
// std 1.0, so its standardized value is always 0 and it contributes nothing.
fn standardization(samples: &[Sample], n_features: usize) -> (Vec<f64>, Vec<f64>) {
    let n = samples.len().max(1) as f64;
    let means: Vec<f64> = (0..n_features)
        .map(|j| samples.iter().map(|s| s.values[j]).sum::<f64>() / n)
        .collect();
    let stds: Vec<f64> = (0..n_features)
        .map(|j| {
            let variance = samples
                .iter()
                .map(|s| (s.values[j] - means[j]).powi(2))
                .sum::<f64>()
                / n;
            let std = variance.sqrt();
            if std > 1e-9 { std } else { 1.0 }
        })
        .collect();
    (means, stds)
}

fn standardize(values: &[f64], means: &[f64], stds: &[f64]) -> Vec<f64> {
    values
        .iter()
        .zip(means.iter().zip(stds))
        .map(|(x, (m, s))| (x - m) / s)
        .collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

// The smallest f64 above `x` (for x in [0, 1]).
fn next_up(x: f64) -> f64 {
    if x >= 1.0 {
        1.0
    } else {
        f64::from_bits(x.to_bits() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny deterministic generator, so the synthetic data is the same every run.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn names(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("f{i}")).collect()
    }

    // Fraud is driven by feature 0 (strongly), feature 1 (mildly, negatively), and
    // not at all by feature 2.
    fn synthetic(n: usize, seed: u64) -> Vec<Sample> {
        let mut rng = Lcg(seed);
        (0..n)
            .map(|_| {
                let x = vec![rng.next() * 4.0, rng.next() * 4.0, rng.next() * 4.0];
                let logit = -4.0 + 2.0 * x[0] - 0.5 * x[1];
                let fraudulent = rng.next() < sigmoid(logit);
                Sample {
                    values: x,
                    fraudulent,
                    weight: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn learns_which_features_matter_and_their_direction() {
        let data = synthetic(3000, 7);
        let model = fit(&data, &names(3), FitOptions::default());

        assert!(model.weights[0] > 1.0, "{:?}", model.weights);
        assert!(model.weights[1] < -0.1, "{:?}", model.weights);
        assert!(
            model.weights[2].abs() < model.weights[0].abs() / 5.0,
            "{:?}",
            model.weights
        );
        assert!(evaluate(&model, &synthetic(1000, 99)).auc > 0.85);
    }

    #[test]
    fn fitting_is_deterministic() {
        let data = synthetic(500, 3);
        let a = fit(&data, &names(3), FitOptions::default());
        let b = fit(&data, &names(3), FitOptions::default());
        assert_eq!(a, b);
    }

    #[test]
    fn the_threshold_meets_the_false_positive_target() {
        let train = synthetic(3000, 11);
        let validation = synthetic(2000, 12);
        let mut model = fit(&train, &names(3), FitOptions::default());

        model.threshold = choose_threshold(&model, &validation, 0.02);
        let metrics = evaluate(&model, &validation);

        assert!(metrics.false_positive_rate <= 0.02, "{metrics:?}");
        assert!(model.threshold >= 0.5);
        assert!(metrics.recall > 0.0, "{metrics:?}");
    }

    #[test]
    fn a_constant_feature_contributes_nothing() {
        let data: Vec<Sample> = synthetic(400, 5)
            .into_iter()
            .map(|mut s| {
                s.values[2] = 1.0;
                s
            })
            .collect();
        let model = fit(&data, &names(3), FitOptions::default());
        let scored = model.score(&[1.0, 1.0, 1.0]).unwrap();
        let constant = scored
            .contributions
            .iter()
            .find(|c| c.feature == "f2")
            .unwrap();
        assert_eq!(constant.contribution, 0.0);
    }

    #[test]
    fn auc_is_rank_based() {
        assert_eq!(auc(&[(0.1, false), (0.9, true)]), 1.0);
        assert_eq!(auc(&[(0.9, false), (0.1, true)]), 0.0);
        assert_eq!(auc(&[(0.5, false), (0.5, true)]), 0.5);
        assert_eq!(
            auc(&[(0.5, false)]),
            0.5,
            "one class: undefined, reported as chance"
        );
    }
}
