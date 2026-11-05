# Fraud and Suspicious Login Detection System Specification

## 1. Purpose

This document specifies a Fraud and Suspicious Login Detection System for an existing Identity Provider (IdP).

The system is responsible for detecting suspicious authentication behavior based on IP address, geolocation, user behavior, login velocity, failed attempts, impossible travel, device context, tenant-level patterns, and administrative security policies.

The initial implementation must prioritize a pragmatic MVP using deterministic rules and risk scoring instead of machine learning. The system should be explainable, auditable, privacy-aware, and safe against excessive false positives.

---

## 2. Scope

### 2.1 In Scope

The system must support:

- Collection of login attempt events.
- Detection of suspicious login attempts.
- Risk score calculation.
- Risk decision output.
- Temporary IP/user blocking.
- Adaptive MFA decision support.
- Security event storage.
- Admin dashboard for monitoring.
- Tenant-aware risk policies.
- Basic geolocation and ASN enrichment.
- Detection of brute force and password spraying.
- Detection of impossible travel.
- Detection of new or unusual login context.
- Audit-friendly explanation of risk signals.

### 2.2 Out of Scope for MVP

The MVP should not include:

- Machine learning models.
- Advanced behavioral biometrics.
- Browser fingerprinting beyond basic normalized user-agent/device data.
- Permanent automatic account bans.
- Paid threat intelligence dependency as a hard requirement.
- Real-time SIEM integration as a requirement.
- Complex anomaly detection pipelines.
- Fully automated incident response workflows.

These features may be added later as part of the advanced roadmap.

---

## 3. High-Level Architecture

The system should be implemented as a Risk Engine module integrated with the existing IdP.

For the MVP, it can be implemented inside the same backend application. Later, it can be extracted into a separate Security/Risk service.

```text
Client Login Request
        |
        v
Identity Provider
        |
        |-- Pre-authentication Risk Check
        |       - IP block check
        |       - rate limit check
        |       - obvious abusive behavior check
        |
        |-- Credential Validation
        |
        |-- Post-authentication Risk Check
        |       - new country
        |       - impossible travel
        |       - new device/user-agent
        |       - user/tenant behavior
        |
        v
Risk Decision
        |
        |-- ALLOW
        |-- ALLOW_WITH_MFA
        |-- BLOCK_TEMPORARILY
        |-- REVIEW
        |
        v
Audit Log + Admin Dashboard
```

---

## 4. Core Concepts

### 4.1 Login Event

A login event represents any authentication attempt, successful or failed.

The system must record both successful and failed login attempts. Failed attempts are essential for detecting brute force, credential stuffing, and password spraying.

### 4.2 Risk Signal

A risk signal is a specific suspicious indicator detected during evaluation.

Examples:

- Too many failed attempts from the same IP.
- Too many failed attempts against the same user.
- Same IP trying many different users.
- Login from a new country.
- Impossible travel.
- Login from a known blocked IP.
- Login from a suspicious ASN.
- Login from a new device/user-agent.

### 4.3 Risk Score

Each signal contributes points to a risk score.

Example:

```text
NEW_COUNTRY                         +20
NEW_DEVICE                          +15
TOO_MANY_FAILED_ATTEMPTS_BY_IP      +30
TOO_MANY_FAILED_ATTEMPTS_BY_USER    +30
PASSWORD_SPRAYING                   +40
IMPOSSIBLE_TRAVEL                   +50
BLOCKED_IP                          +80
KNOWN_TOR_OR_PROXY                  +40
```

### 4.4 Risk Level

The score should map to a human-readable level.

```text
0-29    LOW
30-59   MEDIUM
60-89   HIGH
90+     CRITICAL
```

### 4.5 Risk Decision

The Risk Engine must return one of the following decisions:

```text
ALLOW
ALLOW_WITH_MFA
BLOCK_TEMPORARILY
REVIEW
```

Recommended initial mapping:

```text
LOW       -> ALLOW
MEDIUM    -> ALLOW
HIGH      -> ALLOW_WITH_MFA
CRITICAL  -> BLOCK_TEMPORARILY
```

The exact mapping should be configurable per tenant.

---

## 5. MVP Requirements

## 5.1 Backend MVP Requirements

### 5.1.1 Login Attempt Collection

The backend must collect and persist login attempt data.

Each login attempt must include:

- Tenant ID.
- User ID, if known.
- Client ID, if known.
- Email/username identifier, preferably normalized and optionally hashed.
- IP address.
- User-Agent.
- Timestamp.
- Credential validation result.
- Failure reason, if applicable.
- Risk score.
- Risk level.
- Risk decision.
- Detected risk signals.

Important: avoid storing raw passwords, MFA codes, authorization codes, refresh tokens, access tokens, or session secrets.

---

### 5.1.2 Pre-Authentication Risk Check

Before validating credentials, the system should perform lightweight checks:

- Is the IP temporarily blocked?
- Is the user temporarily blocked?
- Has this IP exceeded the allowed attempt rate?
- Is this IP targeting many users?
- Is the tenant under a high-volume attack?

This protects the authentication endpoint from abuse and avoids unnecessary expensive operations.

Recommended decisions:

- If IP is temporarily blocked: `BLOCK_TEMPORARILY`.
- If rate limit is exceeded: `BLOCK_TEMPORARILY` or delayed response.
- If suspicious but not severe: continue to credential validation and record risk signal.

---

### 5.1.3 Post-Authentication Risk Check

After credential validation, the system should perform deeper checks.

For failed credentials:

- Increment failed attempt counters.
- Detect brute force.
- Detect password spraying.
- Store failure event.
- Optionally block IP/user temporarily if thresholds are exceeded.

For valid credentials:

- Check new country.
- Check new city/region.
- Check impossible travel.
- Check new user-agent/device context.
- Check known suspicious IP/ASN.
- Calculate final risk score.
- Decide whether MFA is required.

---

### 5.1.4 Redis-Based Counters

Redis should be used for short-lived counters and temporary state.

Recommended keys:

```text
risk:failed_ip:{tenant_id}:{ip}
risk:failed_user:{tenant_id}:{user_id}
risk:ip_users:{tenant_id}:{ip}
risk:blocked_ip:{tenant_id}:{ip}
risk:blocked_user:{tenant_id}:{user_id}
risk:tenant_failures:{tenant_id}
```

Recommended TTLs:

```text
Failed IP counter:          10-15 minutes
Failed user counter:        10-15 minutes
IP users set:               10-15 minutes
Temporary IP block:         15-60 minutes
Temporary user block:       15-60 minutes
Tenant attack counter:      10-15 minutes
```

Redis should be used for decisions that require real-time speed. PostgreSQL should be used for durable audit history.

---

### 5.1.5 PostgreSQL Storage

PostgreSQL should store historical events and risk decisions.

Recommended tables:

```sql
CREATE TABLE login_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    user_id UUID NULL,
    client_id UUID NULL,

    username_normalized TEXT NULL,
    username_hash TEXT NULL,

    ip_address INET NOT NULL,
    ip_hash TEXT NULL,

    user_agent TEXT NULL,
    user_agent_hash TEXT NULL,
    device_fingerprint_hash TEXT NULL,

    country_code TEXT NULL,
    region TEXT NULL,
    city TEXT NULL,
    latitude DOUBLE PRECISION NULL,
    longitude DOUBLE PRECISION NULL,
    asn INTEGER NULL,
    isp TEXT NULL,

    credentials_valid BOOLEAN NOT NULL,
    success BOOLEAN NOT NULL,
    failure_reason TEXT NULL,

    risk_score INTEGER NOT NULL DEFAULT 0,
    risk_level TEXT NOT NULL,
    decision TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

```sql
CREATE TABLE login_risk_signals (
    id UUID PRIMARY KEY,
    login_event_id UUID NOT NULL REFERENCES login_events(id),
    tenant_id UUID NOT NULL,

    signal_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    points INTEGER NOT NULL,
    metadata_json JSONB NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

```sql
CREATE TABLE security_blocks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,

    block_type TEXT NOT NULL,
    ip_address INET NULL,
    user_id UUID NULL,

    reason TEXT NOT NULL,
    source TEXT NOT NULL,

    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL
);
```

```sql
CREATE TABLE tenant_risk_policies (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL UNIQUE,

    enabled BOOLEAN NOT NULL DEFAULT true,

    failed_ip_threshold INTEGER NOT NULL DEFAULT 10,
    failed_user_threshold INTEGER NOT NULL DEFAULT 5,
    password_spraying_user_threshold INTEGER NOT NULL DEFAULT 10,

    medium_risk_threshold INTEGER NOT NULL DEFAULT 30,
    high_risk_threshold INTEGER NOT NULL DEFAULT 60,
    critical_risk_threshold INTEGER NOT NULL DEFAULT 90,

    require_mfa_on_high_risk BOOLEAN NOT NULL DEFAULT true,
    block_on_critical_risk BOOLEAN NOT NULL DEFAULT true,

    temporary_block_minutes INTEGER NOT NULL DEFAULT 30,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Recommended indexes:

```sql
CREATE INDEX idx_login_events_tenant_created_at
ON login_events (tenant_id, created_at DESC);

CREATE INDEX idx_login_events_tenant_user_created_at
ON login_events (tenant_id, user_id, created_at DESC);

CREATE INDEX idx_login_events_tenant_ip_created_at
ON login_events (tenant_id, ip_address, created_at DESC);

CREATE INDEX idx_login_events_tenant_risk_level_created_at
ON login_events (tenant_id, risk_level, created_at DESC);

CREATE INDEX idx_login_risk_signals_tenant_signal_created_at
ON login_risk_signals (tenant_id, signal_type, created_at DESC);

CREATE INDEX idx_security_blocks_tenant_ip_expires
ON security_blocks (tenant_id, ip_address, expires_at);
```

---

## 6. MVP Detection Rules

### 6.1 Rule: Temporarily Blocked IP

#### Description

If the IP address is currently blocked for the tenant, the login attempt must be denied.

#### Signal

```text
BLOCKED_IP
```

#### Suggested Points

```text
+80
```

#### Decision

```text
BLOCK_TEMPORARILY
```

---

### 6.2 Rule: Too Many Failed Attempts by IP

#### Description

Detects brute force attempts from a single IP.

#### Example

```text
Same IP failed 10 times in 10 minutes.
```

#### Signal

```text
TOO_MANY_FAILED_ATTEMPTS_BY_IP
```

#### Suggested Points

```text
+30
```

#### Suggested Action

- Medium severity: require MFA on valid credentials.
- High severity: temporarily block IP.

---

### 6.3 Rule: Too Many Failed Attempts by User

#### Description

Detects repeated failed login attempts against the same user.

#### Example

```text
Same user had 5 failed login attempts in 10 minutes.
```

#### Signal

```text
TOO_MANY_FAILED_ATTEMPTS_BY_USER
```

#### Suggested Points

```text
+30
```

#### Suggested Action

- Require MFA after successful credential validation.
- Optionally delay further attempts.
- Avoid aggressive user lockout because attackers can abuse lockouts for denial-of-service.

---

### 6.4 Rule: Password Spraying

#### Description

Detects one IP trying to authenticate against many different users.

#### Example

```text
Same IP tried 10 different users in 10 minutes.
```

#### Signal

```text
PASSWORD_SPRAYING
```

#### Suggested Points

```text
+40
```

#### Suggested Action

- Temporarily block IP.
- Alert tenant administrator.
- Increase tenant attack counter.

---

### 6.5 Rule: New Country for User

#### Description

Detects a successful credential validation from a country not previously observed for the user.

#### Signal

```text
NEW_COUNTRY
```

#### Suggested Points

```text
+20
```

#### Suggested Action

```text
ALLOW_WITH_MFA
```

This should generally not cause automatic blocking by itself.

---

### 6.6 Rule: Impossible Travel

#### Description

Detects a successful login from a geographically distant location shortly after a previous successful login.

#### Example

```text
10:00 - Successful login in São Paulo.
10:30 - Successful login in London.
```

#### Signal

```text
IMPOSSIBLE_TRAVEL
```

#### Suggested Points

```text
+50
```

#### Suggested MVP Logic

```text
Distance > 1000 km and elapsed time < 1 hour  -> suspicious
Distance > 5000 km and elapsed time < 6 hours -> highly suspicious
```

#### Suggested Action

```text
ALLOW_WITH_MFA
```

For very high scores, the decision may be `BLOCK_TEMPORARILY`.

---

### 6.7 Rule: New Device/User-Agent

#### Description

Detects login from a new browser/OS/device context.

For MVP, this should be based on normalized, non-invasive user-agent parsing.

#### Signal

```text
NEW_DEVICE
```

#### Suggested Points

```text
+15
```

#### Suggested Action

- Increase score.
- Do not block by itself.
- Combine with other signals.

---

### 6.8 Rule: Tenant-Wide Attack Spike

#### Description

Detects a sudden increase in failed logins across a tenant.

#### Example

```text
Tenant received 100 failed login attempts in 10 minutes.
```

#### Signal

```text
TENANT_ATTACK_SPIKE
```

#### Suggested Points

```text
+30
```

#### Suggested Action

- Notify administrators.
- Increase CAPTCHA/MFA requirements if supported.
- Apply stricter IP-level blocking temporarily.

---

## 7. Risk Engine API Design

### 7.1 Internal Evaluation Input

Example Rust-like structure:

```rust
pub struct RiskEvaluationInput {
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub client_id: Option<Uuid>,

    pub username_normalized: Option<String>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,

    pub credentials_valid: bool,
    pub failure_reason: Option<String>,

    pub timestamp: DateTime<Utc>,
}
```

### 7.2 Internal Evaluation Result

```rust
pub struct RiskEvaluationResult {
    pub score: u16,
    pub level: RiskLevel,
    pub decision: RiskDecision,
    pub signals: Vec<RiskSignal>,
}
```

### 7.3 Risk Decision Enum

```rust
pub enum RiskDecision {
    Allow,
    AllowWithMfa,
    BlockTemporarily,
    Review,
}
```

### 7.4 Risk Signal

```rust
pub struct RiskSignal {
    pub signal_type: RiskSignalType,
    pub severity: RiskSeverity,
    pub points: u16,
    pub metadata: serde_json::Value,
}
```

---

## 8. Integration with the Existing IdP

### 8.1 Login Flow

The IdP login flow should call the Risk Engine in two stages.

```text
1. Receive login request.
2. Extract tenant, client, username, IP, and user-agent.
3. Run pre-authentication risk check.
4. If blocked, return a generic authentication failure response.
5. Validate credentials.
6. Run post-authentication risk check.
7. Store login event and risk signals.
8. If credentials are invalid, return generic failure.
9. If decision is ALLOW, create session / continue OAuth flow.
10. If decision is ALLOW_WITH_MFA, redirect to MFA challenge.
11. If decision is BLOCK_TEMPORARILY, deny login.
12. If decision is REVIEW, allow or require MFA depending on tenant policy.
```

### 8.2 Response Safety

The public login response must avoid revealing whether:

- The username exists.
- The password was correct.
- The IP was blocked.
- The account is being attacked.
- A specific risk rule was triggered.

Recommended public message:

```text
Invalid credentials or login temporarily unavailable. Please try again later.
```

The detailed reason should only appear in server logs and the admin dashboard.

---

## 9. Admin Dashboard Requirements

The Admin Dashboard should allow tenant administrators and system administrators to monitor login risk.

## 9.1 MVP Dashboard Pages

### 9.1.1 Security Overview

The overview page should show:

- Total login attempts in selected period.
- Successful logins.
- Failed logins.
- Blocked attempts.
- High-risk attempts.
- Critical-risk attempts.
- Top suspicious IPs.
- Top attacked users.
- Login attempts by country.
- Risk trend over time.

Recommended cards:

```text
Total Attempts
Failed Attempts
Blocked Attempts
High Risk Logins
Password Spraying Events
Top Suspicious IP
```

---

### 9.1.2 Login Events Table

The login events table should support:

- Date range filter.
- Tenant filter, for system admins.
- User filter.
- Client filter.
- IP filter.
- Country filter.
- Risk level filter.
- Decision filter.
- Success/failure filter.
- Signal type filter.

Columns:

```text
Time
User
Client
IP
Country
City
Success
Risk Score
Risk Level
Decision
Signals
```

Each row should open a details drawer/page.

---

### 9.1.3 Login Event Details

The event details page should show:

- Basic event information.
- User information.
- Client/application.
- IP address.
- Geolocation.
- ASN/ISP.
- User-agent.
- Success/failure status.
- Risk score.
- Risk decision.
- Triggered signals.
- Explanation of each signal.
- Related events from same IP.
- Related events for same user.
- Admin actions.

Example explanation:

```text
This login was classified as HIGH risk because:
- The IP failed 12 login attempts in the last 10 minutes.
- The login came from a country not previously used by this user.
- The device context was not recognized.
```

---

### 9.1.4 Suspicious IPs Page

This page should show:

- IP address.
- Countries/cities observed.
- Number of attempts.
- Number of failed attempts.
- Number of distinct users targeted.
- First seen.
- Last seen.
- Current block status.
- Risk signals associated with the IP.

Admin actions:

- Temporarily block IP.
- Unblock IP.
- Add IP to trusted list.
- Add note.
- Export events.

---

### 9.1.5 Attacked Users Page

This page should show users with suspicious activity:

- User.
- Number of failed attempts.
- Number of source IPs.
- Last successful login.
- Last failed login.
- Risk level.
- MFA enabled status.
- Account status.

Admin actions:

- Require password reset.
- Require MFA enrollment.
- Revoke sessions.
- Temporarily block login.
- View related events.

---

### 9.1.6 Risk Policies Page

Tenant administrators should be able to configure risk policy values.

Configurable values:

```text
Enable/disable Risk Engine
Failed attempts by IP threshold
Failed attempts by user threshold
Password spraying distinct user threshold
Temporary block duration
Risk thresholds
Require MFA on high risk
Block on critical risk
Allowed countries
Blocked countries
Trusted IPs
Blocked IPs
```

Important: dangerous policy changes should require confirmation and should be audit logged.

---

### 9.1.7 Map View

The dashboard may include a map visualization.

MVP map features:

- Plot recent login attempts by approximate city/region.
- Color/group by risk level.
- Filter by success/failure.
- Filter by tenant.
- Filter by date range.
- Show aggregated counts, not exact user home locations.

Privacy recommendation:

- Prefer city-level or region-level visualization.
- Avoid showing overly precise latitude/longitude for end users.
- Do not expose full personal location history to low-privilege admins.

---

## 10. Frontend Best Practices

### 10.1 Recommended Frontend Stack

Suggested stack:

```text
Framework: Next.js + TypeScript
UI: shadcn/ui, Radix UI, or equivalent
Charts: Recharts, ECharts, or similar
Map: Leaflet, MapLibre, or Mapbox
Data fetching: TanStack Query
Tables: TanStack Table
Forms: React Hook Form + Zod
```

### 10.2 Dashboard UX Principles

The dashboard should be:

- Fast to scan.
- Filterable.
- Explainable.
- Action-oriented.
- Safe against accidental destructive actions.

Every risk decision should be explainable.

Admins should be able to answer:

```text
What happened?
Who was affected?
Which IP was involved?
Why was it considered suspicious?
What action was taken?
What should I do next?
```

### 10.3 Visual Design

Recommended visual components:

- KPI cards for high-level metrics.
- Time-series chart for attempts over time.
- Risk distribution chart.
- Login event table.
- IP/user detail drawers.
- Map for geolocation analysis.
- Policy configuration forms.
- Admin action audit trail.

### 10.4 Dashboard Permissions

Access should be role-based.

Suggested roles:

```text
SECURITY_VIEWER
SECURITY_ANALYST
TENANT_ADMIN
SYSTEM_ADMIN
```

Suggested permissions:

```text
View security dashboard
View login event details
View IP addresses
Manage risk policies
Block/unblock IPs
Force MFA
Revoke sessions
Export events
```

Sensitive fields such as full IP addresses and user identifiers should be hidden or masked for lower-privilege roles.

---

## 11. API Endpoints for Admin Dashboard

Suggested endpoints:

```http
GET /admin/security/overview
GET /admin/security/login-events
GET /admin/security/login-events/{id}
GET /admin/security/suspicious-ips
GET /admin/security/attacked-users
GET /admin/security/risk-policies
PUT /admin/security/risk-policies
POST /admin/security/blocks/ip
DELETE /admin/security/blocks/ip/{id}
POST /admin/security/users/{user_id}/require-mfa
POST /admin/security/users/{user_id}/revoke-sessions
```

### 11.1 Example: Security Overview Response

```json
{
  "period": {
    "from": "2026-07-01T00:00:00Z",
    "to": "2026-07-02T00:00:00Z"
  },
  "metrics": {
    "total_attempts": 1500,
    "successful_logins": 1200,
    "failed_logins": 300,
    "blocked_attempts": 35,
    "high_risk_attempts": 42,
    "critical_risk_attempts": 8
  },
  "top_suspicious_ips": [
    {
      "ip": "203.0.113.10",
      "attempts": 120,
      "failed_attempts": 118,
      "distinct_users": 45,
      "risk_score": 95
    }
  ]
}
```

---

## 12. IP Enrichment

### 12.1 MVP Enrichment Fields

The MVP should enrich IP addresses with:

```text
Country
Region
City
Latitude
Longitude
ASN
ISP / Organization
```

Optional:

```text
VPN/proxy/Tor detection
Hosting provider detection
Known malicious IP status
```

### 12.2 Enrichment Strategy

Recommended MVP options:

- Use a local GeoIP database for country/city/ASN lookup.
- Cache IP enrichment results.
- Avoid calling external APIs synchronously during every login.
- Use asynchronous background enrichment when possible.
- Store only the fields needed for security decisions and dashboard views.

### 12.3 Handling Private and Local IPs

The system must handle:

```text
127.0.0.1
::1
10.0.0.0/8
172.16.0.0/12
192.168.0.0/16
fc00::/7
```

For private/local IPs, geolocation should be skipped.

---

## 13. Privacy and Data Protection

The system will process sensitive security metadata.

Best practices:

- Store only what is necessary.
- Apply retention policies.
- Avoid precise location when approximate location is enough.
- Hash or tokenize IPs where full IP is not required.
- Restrict access to raw IP addresses.
- Avoid storing invasive browser fingerprints.
- Audit all admin access to security event details.
- Do not expose risk internals to unauthenticated users.
- Use tenant isolation for all queries.
- Encrypt sensitive data at rest if required by the environment.
- Ensure exports are permission-controlled.

### 13.1 Suggested Retention

Example retention policy:

```text
Detailed login events: 90-180 days
Aggregated analytics: 12-24 months
Security blocks: until expiration + 90 days
Admin audit logs: 1-3 years
```

Retention should be configurable based on legal and business requirements.

---

## 14. Security Best Practices

### 14.1 Authentication Endpoint Protection

The authentication endpoint should have:

- Rate limiting.
- Login throttling.
- Generic error messages.
- Constant-time password verification where applicable.
- Strong password hashing.
- MFA support.
- Audit logging.
- Bot protection if necessary.
- Abuse detection by IP, user, tenant, and client.

### 14.2 Avoid User Enumeration

The system must not reveal whether a user exists.

Bad response:

```text
This email does not exist.
```

Better response:

```text
Invalid credentials.
```

### 14.3 Avoid Lockout Abuse

User lockout can be abused as a denial-of-service attack.

Prefer:

- Temporary IP blocks.
- Progressive delays.
- MFA challenges.
- CAPTCHA after threshold.
- Soft account protection instead of aggressive permanent lockout.

### 14.4 Explainability

Every risk decision must be explainable internally.

Example:

```json
{
  "decision": "ALLOW_WITH_MFA",
  "risk_score": 75,
  "signals": [
    {
      "type": "NEW_COUNTRY",
      "points": 20
    },
    {
      "type": "IMPOSSIBLE_TRAVEL",
      "points": 50
    },
    {
      "type": "NEW_DEVICE",
      "points": 15
    }
  ]
}
```

### 14.5 Audit Logging

The following events must be auditable:

- Login success.
- Login failure.
- MFA challenge required.
- MFA challenge passed.
- MFA challenge failed.
- IP temporarily blocked.
- IP manually blocked.
- IP manually unblocked.
- Risk policy changed.
- Admin viewed sensitive login details.
- Admin exported security data.
- Sessions revoked.
- MFA required by admin.

---

## 15. Observability

The system should expose metrics for monitoring.

Recommended metrics:

```text
auth_login_attempts_total
auth_login_success_total
auth_login_failure_total
auth_login_blocked_total
auth_high_risk_login_total
auth_mfa_required_by_risk_total
auth_password_spraying_detected_total
auth_impossible_travel_detected_total
risk_engine_evaluation_duration_ms
risk_engine_redis_errors_total
risk_engine_ip_enrichment_errors_total
```

Recommended logs:

- Structured JSON logs.
- Correlation ID.
- Tenant ID.
- Event ID.
- Risk decision.
- Risk signals.
- Avoid sensitive values.

Recommended alerts:

- Sudden increase in failed logins.
- Password spraying detected.
- Many critical-risk events.
- Redis unavailable.
- IP enrichment provider unavailable.
- Abnormally high block rate.

---

## 16. Testing Strategy

### 16.1 Unit Tests

Test each risk rule independently.

Examples:

- Same IP exceeds failed attempt threshold.
- Same user exceeds failed attempt threshold.
- Same IP targets many users.
- New country is detected.
- Impossible travel is detected.
- Blocked IP is denied.
- New device adds score but does not block alone.

### 16.2 Integration Tests

Test full login flow:

- Invalid credentials create login event.
- Valid credentials with low risk are allowed.
- Valid credentials with high risk require MFA.
- Blocked IP is denied before credential validation.
- Risk signals are persisted.
- Redis counters expire correctly.

### 16.3 Frontend Tests

Test dashboard behavior:

- Filters work correctly.
- Event details render signals.
- Admin cannot perform actions without permission.
- Policy form validates values.
- Map handles empty or aggregated data.

### 16.4 Security Tests

Test:

- User enumeration resistance.
- Tenant isolation.
- Unauthorized access to security events.
- Rate limit bypass attempts.
- Input validation for IP filters and query parameters.
- Export permission enforcement.

---

## 17. MVP Implementation Plan

### Phase 1: Data Model and Event Collection

Deliverables:

- `login_events` table.
- `login_risk_signals` table.
- Event recording for successful and failed login attempts.
- Basic admin list of login events.

### Phase 2: Redis Counters and Basic Rules

Deliverables:

- Failed attempts by IP.
- Failed attempts by user.
- Password spraying detection.
- Temporary IP block.
- Risk score calculation.

### Phase 3: IP Enrichment

Deliverables:

- Country/city/ASN enrichment.
- Cached enrichment results.
- New country detection.
- Dashboard country filters.

### Phase 4: Adaptive MFA

Deliverables:

- `ALLOW_WITH_MFA` decision support.
- IdP integration to redirect high-risk successful logins to MFA.
- Admin visibility into MFA-required risk decisions.

### Phase 5: Impossible Travel

Deliverables:

- Last successful login lookup.
- Distance calculation.
- Impossible travel rule.
- Event detail explanation.

### Phase 6: Admin Dashboard MVP

Deliverables:

- Security overview.
- Login event table.
- Login event details.
- Suspicious IPs page.
- Basic risk policy page.
- Map view with aggregated geolocation.

---

## 18. Advanced Roadmap

After the MVP, the system can evolve with the following capabilities.

## 18.1 Advanced Detection

Add:

- Credential stuffing detection.
- Known leaked credential detection.
- Tor/VPN/proxy detection.
- Hosting provider/datacenter detection.
- ASN reputation.
- Device trust scoring.
- User behavior baselines.
- Time-of-day anomaly detection.
- Client/application-specific risk rules.
- Tenant-specific allow/block country policies.
- Velocity checks across multiple dimensions.
- Session hijacking indicators.

## 18.2 Threat Intelligence

Add integrations with:

- IP reputation providers.
- Abuse databases.
- Tor exit node lists.
- Known proxy/VPN datasets.
- Internal deny lists.
- SIEM/SOC tools.

Important: external intelligence should enrich decisions, not blindly control them.

## 18.3 Admin Response Workflows

Add:

- Case management.
- Security incident timeline.
- Assign investigation to admin.
- Add notes to suspicious IP/user.
- Mark false positive.
- Mark confirmed attack.
- Export incident report.
- Notify affected users.
- Force password reset.
- Revoke all sessions for user/tenant.
- Require MFA enrollment.

## 18.4 Policy Engine

Add more flexible policy rules:

```text
If country is outside allowed list -> require MFA
If ASN is hosting provider -> require MFA
If risk score >= 90 -> block
If user is privileged and risk score >= 40 -> require MFA
If client is high-sensitivity app -> stricter thresholds
```

Long-term, the policy system can be implemented as:

- Database-driven rules.
- Expression language.
- JSON-based rule definitions.
- Open Policy Agent integration.

## 18.5 Machine Learning

Only consider ML after enough high-quality labeled data exists.

Useful ML targets:

- Login anomaly detection.
- User behavior clustering.
- Suspicious IP clustering.
- False-positive reduction.
- Attack campaign detection.

Requirements before ML:

- High-quality event data.
- Clear labels.
- Admin feedback loop.
- Monitoring for model drift.
- Explainability strategy.
- Manual override capability.

## 18.6 SIEM and Webhooks

Add:

- Webhooks for high-risk events.
- SIEM export.
- OpenTelemetry traces.
- Syslog integration.
- Slack/Teams alerts.
- Email alerts.
- JSON event stream.

Example webhook event:

```json
{
  "event_type": "security.login.high_risk",
  "tenant_id": "00000000-0000-0000-0000-000000000000",
  "login_event_id": "00000000-0000-0000-0000-000000000000",
  "risk_score": 85,
  "risk_level": "HIGH",
  "decision": "ALLOW_WITH_MFA",
  "signals": ["NEW_COUNTRY", "IMPOSSIBLE_TRAVEL"]
}
```

---

## 19. Recommended Defaults for MVP

```text
Failed attempts by IP threshold:              10 attempts / 10 minutes
Failed attempts by user threshold:            5 attempts / 10 minutes
Password spraying threshold:                  10 distinct users / 10 minutes
Temporary IP block duration:                  30 minutes
Temporary user soft block duration:           15 minutes
Medium risk threshold:                        30
High risk threshold:                          60
Critical risk threshold:                      90
High risk action:                             Require MFA
Critical risk action:                         Temporary block
Detailed event retention:                     90 days
Aggregated analytics retention:               12 months
```

These values should be configurable per tenant.

---

## 20. Non-Functional Requirements

### 20.1 Performance

The Risk Engine should be fast enough to run during login.

Targets:

```text
Pre-authentication risk check: < 30 ms excluding network latency
Post-authentication risk check: < 100 ms excluding external enrichment
Dashboard list queries: < 500 ms for common filters
```

Do not synchronously call slow external APIs during login.

### 20.2 Availability

If Redis is unavailable:

- The IdP should not fully stop accepting logins.
- The system should degrade gracefully.
- High-confidence PostgreSQL-based checks may still run.
- The incident should trigger operational alerts.

If IP enrichment is unavailable:

- Continue login flow.
- Mark enrichment as unavailable.
- Avoid blocking solely because enrichment failed.

### 20.3 Scalability

The system should support:

- High login volume.
- Tenant isolation.
- Partitioning or archiving of old login events.
- Efficient filtering by tenant, time, user, IP, and risk level.

For high-volume systems, consider:

- Table partitioning by month.
- Background aggregation jobs.
- Materialized views for dashboard metrics.
- Event streaming pipeline.

---

## 21. Example End-to-End Scenario

### Scenario

A valid user login is attempted from a new country shortly after a previous login from Brazil.

### Input

```text
Previous successful login:
- São Paulo, Brazil
- 2026-07-01T10:00:00Z

Current login:
- London, United Kingdom
- 2026-07-01T10:30:00Z
- Credentials valid
- New user-agent
```

### Detected Signals

```text
NEW_COUNTRY           +20
IMPOSSIBLE_TRAVEL     +50
NEW_DEVICE            +15
```

### Final Risk

```text
Risk Score: 85
Risk Level: HIGH
Decision: ALLOW_WITH_MFA
```

### Admin Explanation

```text
This login was marked as HIGH risk because it came from a new country, used a new device context, and occurred too soon after a previous successful login from a geographically distant location.
```

---

## 22. Final Recommendation

The MVP should focus on deterministic, explainable risk detection:

```text
1. Store every login attempt.
2. Use Redis for temporary counters.
3. Detect brute force by IP.
4. Detect brute force by user.
5. Detect password spraying.
6. Enrich IP with country/city/ASN.
7. Detect new country.
8. Detect impossible travel.
9. Calculate risk score.
10. Require MFA for high-risk logins.
11. Temporarily block critical abuse.
12. Provide an admin dashboard for monitoring and action.
```

This approach provides real security value without overengineering. It also creates a strong foundation for advanced features such as threat intelligence, SIEM integration, policy engines, and machine learning.

---

## 23. Reference Guidance

This specification is aligned with widely used security guidance:

- OWASP Authentication Cheat Sheet: authentication hardening, login throttling, and generic error messages.
- OWASP Multifactor Authentication Cheat Sheet: risk-based authentication and adaptive MFA.
- OWASP Logging Cheat Sheet: structured security event logging.
- OWASP Top 10 Security Logging and Monitoring Failures: importance of logging, monitoring, and alerting.
- NIST SP 800-63B Digital Identity Guidelines: authentication lifecycle and risk-aware authentication practices.

