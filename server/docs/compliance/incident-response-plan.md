# Incident Response & Breach Notification Plan

> **Work item:** WI-10 · **Frameworks:** GDPR Art. 33/34 · LGPD Art. 48 · ISO A.5.24–A.5.28 · SOC 2 CC7.3–7.5
> **Owner:** DPO + Security lead · **Review:** annually + after every incident (post-mortem).

## Roles

- **Incident Commander (IC):** coordinates response.
- **DPO / Encarregado:** owns regulator + data-subject notification decisions/timing.
- **Engineering on-call:** containment, eradication, recovery.
- **Comms:** customer/tenant and public messaging.

## Severity levels

| Sev | Definition | Example |
|---|---|---|
| SEV1 | Confirmed breach of Restricted data / full compromise | Signing-key or credential-store exfiltration |
| SEV2 | Likely breach / significant service abuse | Refresh-token reuse spike, account takeover cluster |
| SEV3 | Contained, no confirmed data exposure | Single failed-login brute force, blocked |

## Lifecycle

1. **Detect** — from monitoring/alerts (see below) or report.
2. **Triage** — IC assigns severity; open incident channel + ticket.
3. **Contain** — revoke tokens/keys, disable accounts/clients, block sources.
4. **Eradicate** — remove cause (patch, rotate secrets, fix config).
5. **Recover** — restore service, monitor for recurrence.
6. **Notify** — see clocks below (DPO decides).
7. **Post-mortem** — blameless review within 5 business days; track actions.

## Notification clocks

- **GDPR:** notify the supervisory authority **within 72 hours** of becoming aware of a
  personal-data breach (Art. 33); notify affected data subjects without undue delay if
  high risk (Art. 34).
- **LGPD:** notify the **ANPD** and affected data subjects in a **reasonable time**
  (Art. 48); ANPD guidance ≈ as soon as practicable.
- Pre-drafted templates: `templates/breach-notification-authority.md` and
  `templates/breach-notification-subject.md` _(to be authored)_.

## Detection hooks (wired to the audit log, WI-02)

The append-only `audit_events` log is the detection substrate. Alert on:

- **Brute force:** burst of `user.login.failure` for one actor/IP.
- **Token theft:** `token.reuse_detected` (already emitted by `token_refresh` —
  family is revoked automatically).
- **Mass export / delete:** spikes of `user.data_exported`, `user.erased`, `data.purged`.
- **Privilege change / access denial:** `role.assigned`, `admin.access_denied` clusters.

Ship `audit_events` (and structured `tracing` logs) to a tamper-evident SIEM and
configure the above as alert rules (see [monitoring-availability.md](monitoring-availability.md)).
