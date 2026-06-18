# Monitoring, Logging & Availability

> **Work item:** WI-11 · **Frameworks:** ISO A.8.15/A.8.16 · SOC 2 CC7.2 + Availability (A1)
> **Owner:** Engineering lead · **Review:** annually.

## Logging

- **Structured application logs** via `tracing`. No Restricted data in logs (see
  [data-classification.md](data-classification.md)).
- **Security/audit events** in `audit_events` (append-only, WI-02). Ship both to a
  central, **tamper-evident** store with retention ≥ the audit floor
  (see [retention-schedule.md](retention-schedule.md)).

## Monitoring & alerting

- Detection rules per [incident-response-plan.md](incident-response-plan.md)
  (brute force, token reuse, mass export/delete, access-denied clusters).
- Operational metrics: request rate/latency/error rate, DB pool health, token-endpoint
  throughput.

## Health & readiness

- `GET /health` — liveness (process up). Implemented in `routes.rs`.
- `GET /ready` — readiness. Currently returns ok; SHOULD be extended to verify DB
  connectivity (`SELECT 1`) before reporting ready. _Enhancement noted._

## Availability (SOC 2 A1)

- **Backups:** automated DB backups with a defined RPO; document frequency + region.
- **Restore testing:** perform and **evidence** a restore at least annually (ISO A.8.13).
  A backup is not a control until a restore has been demonstrated.
- **SLO:** define uptime objective and error budget; track against it.
- **Capacity / resilience:** stateless app instances behind a load balancer; DB with
  automated failover where available.
