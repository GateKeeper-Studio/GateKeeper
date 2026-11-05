# Outbound Webhooks

How other services learn about user lifecycle changes in this IdP, so they can keep
their own user tables in sync.

Gatekeeper authenticates for **separate applications**, often owned by other teams.
Those applications need to know when a user appears, changes, or is removed. This is
that mechanism: a **transactional outbox** feeding **HMAC-signed HTTPS deliveries**.

For the layered design see [`ARCHITECTURE.md`](ARCHITECTURE.md); for the login flows
that produce these events see [`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md).

> **Building the consuming side?** Read
> [`WEBHOOK-CONSUMER-GUIDE.md`](WEBHOOK-CONSUMER-GUIDE.md) instead — this document is
> the producer-side design; that one is the step-by-step integration guide (receiver
> algorithm, verification in five languages, idempotent projection, rotation, backfill).

---

## 1. Why an outbox, and why HTTP

**The outbox is the important part.** Calling a consumer's endpoint *after*
`uow.commit()` loses the event if the process dies in between; calling it *before*
announces a user that a rollback then erases. Both are the classic dual-write bug.
The fix is that the event row is written **in the same transaction as the change it
describes** — an `outbox_events` row exists if and only if that change committed.

**HTTP is the integration contract.** Consumers are third parties. Putting them on a
broker would mean issuing broker credentials per consumer, requiring network
reachability to it, a client library in their language, and their outage becoming our
backlog. A broker remains an option *behind* the outbox later: implementing
`WebhookSender` (`application/ports/webhook_sender.rs`) with an AMQP/Kafka publisher
would not touch a single use case.

---

## 2. Events

| Type | Emitted when |
|---|---|
| `user.created` | Self-registration, admin creation, or first social sign-in that **provisions** a new account |
| `user.updated` | Profile/name edit, custom-field edit, or email verification flipping `email_verified` |
| `user.deleted` | Admin deletes a tenant user |

A social sign-in that **links** a provider to an account the tenant already had emits
nothing: no user was created, and a `user.created` there would make every subscriber
insert a duplicate.

The set is closed (`domain/entities/domain_event_type.rs`); a subscription's
`event_types` filter is validated against it, so a typo is a 400 rather than a
subscription that silently never fires.

---

## 3. The wire contract

```http
POST <subscription.url>
Content-Type: application/json
User-Agent: Gatekeeper-Webhook/1
X-Gatekeeper-Event-Id: 0193c8f1-1c2a-7d3e-9f04-2b6a5c8d1e7f
X-Gatekeeper-Event-Type: user.created
X-Gatekeeper-Timestamp: 1786060000
X-Gatekeeper-Signature: v1=3f1b0e...
```

```json
{
  "id": "0193c8f1-1c2a-7d3e-9f04-2b6a5c8d1e7f",
  "type": "user.created",
  "occurred_at": "2026-08-06T20:00:00+00:00",
  "tenant_id": "0193c8f0-...",
  "data": {
    "user_id": "0193c8f1-...",
    "email": "ada@example.com",
    "email_verified": true,
    "display_name": "Ada",
    "first_name": "Ada",
    "last_name": "Lovelace",
    "is_active": true,
    "source": "self_registration",
    "created_at": "2026-08-06T20:00:00+00:00",
    "updated_at": null
  }
}
```

`source` is `self_registration` | `admin` | `external_provider`, and is `null` on
`user.updated`. A `user.deleted` body carries only `data.user_id` — there is no user
left to describe, and promising more would be a lie by the time a retry goes out
hours later.

A **test** delivery (`POST …/webhooks/{id}/test`) is identical but carries
`"test": true` and a nil `user_id`, so it cannot be mistaken for a real person.

---

## 4. Verifying the signature

The signature is `HMAC-SHA256(secret, "<timestamp>.<raw body>")`, hex-encoded.

Signing the timestamp *with* the body is what bounds replay: an attacker cannot move a
captured signature onto a fresh timestamp. **Reject a skew over ±5 minutes** and
**compare in constant time**.

```python
import hashlib, hmac, time

def verify(secret: str, headers, raw_body: bytes) -> bool:
    timestamp = headers["X-Gatekeeper-Timestamp"]
    received  = headers["X-Gatekeeper-Signature"].removeprefix("v1=")

    # Replay window. Without this the signature is valid forever.
    if abs(time.time() - int(timestamp)) > 300:
        return False

    expected = hmac.new(
        secret.encode(),
        timestamp.encode() + b"." + raw_body,
        hashlib.sha256,
    ).hexdigest()

    # compare_digest, never ==, to avoid leaking the answer by timing.
    return hmac.compare_digest(expected, received)
```

Sign the **raw bytes as received** — do not re-serialize the JSON first, or key
ordering will differ and the signature will never match.

---

## 5. Delivery semantics — read this before writing a consumer

**Delivery is at-least-once. Consumers MUST deduplicate on `id`.** A consumer that
2xx's after a network blip we never saw will be sent the same event again. Store
processed ids, or make the handler idempotent (upsert, not insert).

**Ordering is not guaranteed.** Deliveries are attempted oldest-first, but a retried
event arrives after ones enqueued later. Order on `occurred_at`, or treat each event
as the full current state rather than a delta.

**Respond quickly.** The request times out after `WEBHOOK_TIMEOUT_SECONDS` (default
10). Acknowledge with a 2xx and process asynchronously; a slow handler earns retries
it does not want.

**Any non-2xx is a failure**, including 3xx — redirects are deliberately not followed
(§7).

### Retry schedule

Six attempts, then the delivery is marked `dead` and kept for inspection:

| Attempt | Delay after previous |
|---|---|
| 1 | immediate |
| 2 | 30s |
| 3 | 2m |
| 4 | 10m |
| 5 | 1h |
| 6 | 6h |

`GET /tenants/{tenant_id}/webhook-deliveries` shows `status`, `attempts`,
`last_status` and `last_error` per delivery — the first place to look when an
endpoint "never received anything".

---

## 6. Admin API

All endpoints are admin-only and tenant-scoped (`require_admin` plus
`can_access_tenant`).

| Method | Path | Notes |
|---|---|---|
| `POST` | `/tenants/{tenant_id}/webhooks` | 201; response carries `secret` **once** |
| `GET` | `/tenants/{tenant_id}/webhooks` | Paged; never includes the secret |
| `GET` | `/tenants/{tenant_id}/webhooks/{id}` | |
| `PUT` | `/tenants/{tenant_id}/webhooks/{id}` | `rotate_secret: true` returns a new `secret` |
| `DELETE` | `/tenants/{tenant_id}/webhooks/{id}` | Cascades pending deliveries |
| `POST` | `/tenants/{tenant_id}/webhooks/{id}/test` | Synchronous; reports what the endpoint returned |
| `GET` | `/tenants/{tenant_id}/webhook-deliveries` | The delivery log |

The signing secret is 32 random bytes, stored AES-256-GCM encrypted, and
**unrecoverable after creation** — same contract as an OAuth client secret. Losing it
means rotating it.

An empty `event_types` array means **every** event type.

---

## 7. Security

**The subscription URL is an SSRF boundary, not a formatting check.** A tenant admin
supplies an address that *our* server then makes an outbound request to, from inside
our network. The highest-value target is the cloud metadata endpoint
(`169.254.169.254`), reachable from inside most VPCs and happy to hand out IAM
credentials. `use_cases/webhook_subscription/validation.rs` therefore refuses:

- any scheme but `https` (plain `http` only to local hosts — loopback, a
  private-range IP literal, a single-label name or a `.localhost` / `.internal` /
  `.local` name such as `host.docker.internal` — and only when
  `WEBHOOK_ALLOW_PRIVATE_TARGETS=true`);
- fragments and userinfo (`https://internal-admin@evil.test/`);
- IP-literal hosts in `10/8`, `172.16/12`, `192.168/16`, `127/8`, `169.254/16`,
  `100.64/10`, `::1`, `fc00::/7`, `fe80::/10`, multicast — **including their
  IPv4-mapped IPv6 forms** (`::ffff:169.254.169.254`), which would otherwise sail
  straight past the IPv4 rules.

Redirects are disabled on the HTTP client, so a public URL cannot 302 us to a blocked
one after passing validation.

> **Known limitation — DNS rebinding.** A hostname that passes validation can still
> *resolve* to a private address later, or simply be an internal DNS name that looks
> public. Closing that fully requires resolving the name and pinning the socket to the
> vetted IP at request time. Treat this as defence in depth against a careless or
> semi-trusted tenant admin, **not** as an airtight boundary against an attacker who
> already controls DNS. Where that matters, run the dispatcher behind an egress proxy
> with its own allow-list.

**PII.** Payloads carry email and names to a third-party endpoint. That is the point
of the feature, but it is why the URL is validated as above, why each subscription
gets its own secret (one compromised consumer cannot forge deliveries to another),
and why plain http is refused for any host that could be on the public internet.

---

## 8. Operations

| Env var | Default | Purpose |
|---|---|---|
| `WEBHOOKS_ENABLED` | `true` | Master switch for the dispatcher |
| `WEBHOOK_POLL_INTERVAL_SECONDS` | `5` | How often the outbox is drained |
| `WEBHOOK_BATCH_SIZE` | `100` | Rows claimed per phase per tick |
| `WEBHOOK_TIMEOUT_SECONDS` | `10` | Per-request timeout |
| `WEBHOOK_ALLOW_PRIVATE_TARGETS` | `false` | Relaxes §7 for local dev / self-hosted |

Turning `WEBHOOKS_ENABLED` off **does not lose events** — the outbox keeps filling and
a dispatcher drains it whenever one runs.

**Running several replicas is safe and requires no coordination.** Both claim queries
use `SELECT … FOR UPDATE SKIP LOCKED`, so two workers never take the same row, and
`UNIQUE (event_id, subscription_id)` on `webhook_deliveries` makes fan-out idempotent
if a worker dies mid-phase.

The dispatcher is the **only** long-running background task in the service. It stops
on SIGTERM after the HTTP server drains, so an in-flight delivery finishes and records
its result rather than being retried on the next boot.

### Schema

- `outbox_events` — the log. `published_at IS NULL` means "not yet fanned out".
- `webhook_subscriptions` — tenant-scoped targets; secret encrypted at rest.
- `webhook_deliveries` — one row per (event, subscription), holding the retry state.
  Kept after success/death as the audit trail behind the delivery log.

Neither table is purged automatically; add retention to
`compliance/purge_expired_data` if the volume warrants it.
