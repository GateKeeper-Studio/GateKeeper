# Statistical risk scoring

A second Risk Engine adapter that adds a **statistical score** to the deterministic rules
of [`fraud-specification.md`](fraud-specification.md). The rules stay in charge. The
model is a logistic regression: linear, so every score breaks down exactly, feature by
feature.

The fraud spec (§2.2, §18.5) deferred ML until there was labeled data, an admin feedback
loop, drift monitoring, an explainability strategy and a manual override. This document
describes each of those as built.

---

## 1. Where it sits

```
RiskEngine port (application/ports/risk_engine.rs)
 ├── DefaultRiskEngine   rules only                       RISK_SCORING=rules  (default)
 └── HybridRiskEngine    rules + statistical score        RISK_SCORING=hybrid
       └── per tenant: risk policy `scoring_mode` = rules | shadow | hybrid
```

- **`RISK_SCORING`** (platform) picks the adapter. `rules` wires exactly today's engine.
  `hybrid` wires `HybridRiskEngine` (`application/services/hybrid_risk_engine.rs`),
  which reuses the rules engine inside it.
- **`scoring_mode`** (per tenant, risk policy page or `PUT …/security/risk-policy`) opts a
  tenant in:

| Mode | What happens |
|---|---|
| `rules` (default) | The rules decide. The feature vector is **recorded** (`login_events.risk_features`), which is how a training dataset accumulates. No model is applied. |
| `shadow` | The model scores every sign-in, and its probability and explanation are stored and shown. **It never changes a decision.** Use it to judge false positives first. |
| `hybrid` | The model may raise a sign-in to **step-up MFA**. |

---

## 2. Guardrails (all in `HybridRiskEngine`)

- **It can only add a step-up.** When `p ≥ threshold`, the model adds a `STATISTICAL_RISK`
  signal worth exactly `high_risk_threshold − rule_score` points. The score lands on the
  tenant's HIGH threshold, and the tenant's own HIGH mapping (`require_mfa_on_high_risk`)
  decides. A tenant that does not step up on HIGH gets no step-up from the model either.
- **It never blocks.** It never adds points once the rules alone are at HIGH or above, so
  it can never reach CRITICAL. It does nothing at all if a policy's HIGH threshold is not
  below CRITICAL.
- **It never lowers a rule decision.** Its points are never negative.
- **Only valid credentials.** A wrong password is never escalated; it fails regardless.
- **Only a trustworthy model.** With any of the following, the rules decide alone and a
  warning is logged:
  - no active model;
  - a different feature schema;
  - malformed parameters;
  - a model older than `RISK_MODEL_MAX_AGE_DAYS`;
  - a failed feature query or score.
- **Fast.** Scoring is in-process arithmetic. It adds four indexed queries: IP history,
  ASN share, ASN seen for the user, and the user's success count. The active model is
  cached for `RISK_MODEL_CACHE_TTL_SECONDS`.

---

## 3. Explainability

The model is linear, so `logit = intercept + Σ contribution_i`, where
`contribution_i = weight_i × (x_i − mean_i) / std_i` in log-odds. Nothing is approximated.

- **Every scored sign-in** stores `login_events.risk_model`:
  `{version, mode, probability, threshold, logit, intercept, would_escalate, applied, contributions}`.
  The contributions are the five largest, each with `feature`, `value` and `contribution`.
- **A step-up the model caused** is a `STATISTICAL_RISK` signal next to the rule signals. It
  carries the same contributions, plus a sentence like:
  *"The statistical model rm-… gave this login an 87% fraud probability (step-up threshold
  80%), driven mostly by ip_failure_ratio_7d, new_device. It can require MFA, never block."*
- **The dashboard login-event page** shows a "Statistical score" card with signed bars per
  feature, and the raw feature vector is in the API response.
- **The model as a whole** is readable too: `train_risk_model list` and the `train` report
  print every weight.

The score and its explanation appear only on the **admin** login-event detail endpoint.
They are not on the list endpoints that `/me/security/login-events` reuses: showing them
to account holders would let an attacker probe the model.

---

## 4. Features (schema v1)

Derived numbers only (`application/services/risk_scoring/features.rs`). **No identifier**
enters the vector: no IP, user, device, tenant or country code. That is what makes it safe
to pool the vectors across tenants into one model, and what stops the model from learning
"this IP" or "this country".

| Feature | Meaning |
|---|---|
| `failed_attempts_ip_10m`, `failed_attempts_user_10m`, `distinct_users_ip_10m`, `tenant_failures_10m` | `log1p` of the counts the velocity rules already compute |
| `new_country`, `new_device` | The same per-user checks as the rules (0/1) |
| `travel_speed` | km/h since the last successful login, capped at 2000, divided by 1000: impossible travel as a number |
| `hours_since_last_login`, `prior_successful_logins` | `log1p`; the success count is capped at 100 |
| `new_asn_for_user` | This network (ASN) was never seen for the user |
| `asn_tenant_share` | The share of the tenant's successful logins over 7 days that came from this ASN |
| `ip_failure_ratio_7d` | Laplace-smoothed failure ratio of this IP in the tenant. This is the internal IP reputation. |
| `geo_unavailable`, `no_login_history` | Explicit indicators, so missing data is never read as "normal" or "abnormal" |

**Bias and VPN false positives.** A whole office on a corporate VPN exiting abroad looks
like "new country" to the rules.

- The geo features are relative to **this user** (new for them), never absolute countries.
- `asn_tenant_share` and `new_asn_for_user` tell the model that a network is ordinary for
  the tenant or the user.
- A model trained on admin verdicts learns that a VPN half the tenant uses is not
  suspicious, whatever country it exits in. A test pins this.
- Shadow mode and the drift report exist to catch the rest before it costs users anything.

Changing a feature's meaning, order or scaling is a **new schema version**. The engine
refuses to serve a model to a vector of another schema.

---

## 5. Labels: the training dataset

`login_event_labels` holds at most one label per login event: `legitimate` or `fraudulent`.

| Source | How | Training weight |
|---|---|---|
| `admin` | The dashboard login-event page ("Mark legitimate" / "Mark fraudulent"), or `PUT /tenants/{t}/security/login-events/{id}/label` with `{label, note?}`; `DELETE` clears it. Audited as `security.login_event.labeled`, in the same transaction. | 1.0 |
| `mfa_passed` | The user passed a step-up MFA the **risk engine** forced | 0.3 |
| `mfa_failed` | That step-up's challenge ran out of attempts. This can also be a user mistyping five times, hence the low weight. | 0.3 |

- **An admin label always wins.** A weak label never overwrites it, in SQL and in
  `LoginEventLabel::may_replace`.
- **Only risk-forced step-ups label anything.** MFA a client requires anyway says nothing
  about risk. The session stores `risk_login_event_id`, the login event that forced the
  step-up.
- **Weak labels are written in the MFA use case's own transaction** (`verify_email_otp`,
  `verify_totp`, `verify_webauthn`).
- **Retention.** Labels and feature vectors live and die with their login events.

---

## 6. Training and updates: `train_risk_model`

```bash
cargo run --bin train_risk_model -- train [--since-days 180] [--target-fpr 0.02] [--l2 0.01]
cargo run --bin train_risk_model -- list
cargo run --bin train_risk_model -- promote <model-id> [--force]
cargo run --bin train_risk_model -- rollback
cargo run --bin train_risk_model -- report [--days 30]
```

**`train`.** Reads every labeled vector of the current schema since `--since-days`, from all
tenants (the global model, features only). Then:

1. Splits by time: the oldest 80% trains, the newest 20% validates, as the model will be
   used.
2. Fits a standardized logistic regression with class balancing, L2 and sample weights.
   The fit is deterministic.
3. Picks the threshold: the lowest probability that keeps the validation false-positive
   rate at or below `--target-fpr`, never below 0.5.
4. Saves a **candidate** and prints its metrics and every weight.

It refuses to fit with fewer than 10 labels of either class.

**Quality gates** (stored with the model):
- at least 200 legitimate and 50 fraudulent labels;
- validation AUC ≥ 0.75;
- false-positive rate at the threshold ≤ the target.

**`promote <id>`.** Makes the candidate the one active model and retires the previous one,
atomically: a partial unique index allows only one active model. A candidate that failed
its gates needs `--force`. Use it only to run that model in shadow mode while labels
accumulate. A model of another feature schema can never be promoted. A promotion reaches
every replica within the cache TTL.

**`rollback`.** Re-activates the previously active model.

**`report`.** The drift check: per model version and tenant mode, over the window:
- sign-ins scored;
- how many would step up, and how many did;
- p50, p90 and p99 probability;
- for sign-ins labeled since: how many fraudulent and how many legitimate ones it would
  have stepped up.

A rising share of flagged legitimate sign-ins is drift.

**Cadence.** Retrain monthly, or after about 100 new admin labels, whichever comes first.
Read the report before promoting. `RISK_MODEL_MAX_AGE_DAYS` (default 90) is the backstop:
past it, the model stops scoring and the rules decide alone until a new model is promoted.
A forgotten retraining fails safe, not stale.

**Getting started:**
1. Set `RISK_SCORING=hybrid`, with every tenant left on `rules`. The vectors accumulate.
2. Label sign-ins: admins review on the login-event page, and risk-forced MFA outcomes
   label themselves.
3. `train`, then read the metrics.
4. `promote`, then move a tenant to `shadow` and watch `report`.
5. Move the tenant to `hybrid`.

---

## 7. Configuration

| Env var | Default | |
|---|---|---|
| `RISK_SCORING` | `rules` | `rules` or `hybrid`. Any other value stops the boot. |
| `RISK_MODEL_MAX_AGE_DAYS` | `90` | Older active models stop scoring. |
| `RISK_MODEL_CACHE_TTL_SECONDS` | `60` | How long a promotion takes to reach each replica. |

---

## 8. Known limits

- **No external IP reputation.** "IP reputation" means internal history: the IP's 7-day
  failure ratio in the tenant. External feeds are fraud spec §18.2.
- **Pooled training.** One tenant's labeled feature vectors (numbers only) shape the model
  that scores other tenants' sign-ins. A per-tenant "do not contribute" switch is a
  possible follow-up.
- **Weak labels are noisy**, hence their 0.3 weight. Admin labels dominate.
- **No per-tenant models.** One global model serves every opted-in tenant.
- **Prior IP blocks are not a feature.** The retention purge deletes expired blocks, so the
  count would almost always be zero. The IP's failure ratio carries that signal.
