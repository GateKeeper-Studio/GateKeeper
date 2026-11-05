# Consuming Gatekeeper Webhooks

How to make **another service** — in any language — receive user lifecycle events
from this identity provider and keep its own user table in sync.

This is the **consumer-side** guide. For the producer design (why an outbox, how the
dispatcher works, SSRF hardening of subscription URLs) see
[`FLOWS-WEBHOOKS.md`](FLOWS-WEBHOOKS.md). Everything here is derived from that
implementation and is the contract you can rely on.

```
┌──────────────────────────┐          ┌────────────────────────┐
│ Gatekeeper (this IdP)    │          │ Your service           │
│                          │          │                        │
│ user write ─┐            │  HTTPS   │  POST /webhooks/…      │
│             ├─ same tx   │  POST    │   1. verify signature  │
│ outbox row ─┘            │ ───────▶ │   2. dedupe on id      │
│                          │  signed  │   3. 2xx immediately   │
│ dispatcher (every 5s)    │  HMAC    │   4. project async     │
└──────────────────────────┘          └────────────────────────┘
                 ▲                                  │
                 └──────── retries on non-2xx ──────┘
                     immediate, 30s, 2m, 10m, 1h, 6h → dead
```

**Contents**

1. [Before you start](#1-before-you-start)
2. [Register a subscription](#2-register-a-subscription)
3. [The wire contract](#3-the-wire-contract)
4. [The receiver algorithm (language-agnostic)](#4-the-receiver-algorithm-language-agnostic)
5. [Reference implementations](#5-reference-implementations)
6. [Projecting events into your own database](#6-projecting-events-into-your-own-database)
7. [Testing end to end](#7-testing-end-to-end)
8. [Rotating the secret without downtime](#8-rotating-the-secret-without-downtime)
9. [Initial backfill and reconciliation](#9-initial-backfill-and-reconciliation)
10. [Troubleshooting](#10-troubleshooting)
11. [Go-live checklist](#11-go-live-checklist)
12. [Appendix A — HMAC test vectors](#appendix-a--hmac-test-vectors)
13. [Appendix B — primitives per language](#appendix-b--primitives-per-language)

---

## 1. Before you start

You need three things:

| Thing | How to get it |
|---|---|
| **Admin bearer token** | `POST /admin/login` with `{"email","password"}` → `{"access_token","token_type","expires_in","refresh_token"}`. Send `Authorization: Bearer <access_token>` on every admin call. Renew via `POST /admin/refresh`. |
| **`tenant_id`** | The tenant whose users you want to mirror. Every subscription is scoped to exactly one tenant — one token from tenant A never describes a user in tenant B. |
| **A public HTTPS endpoint** | Plain `http` is refused unless the target is loopback **and** the operator set `WEBHOOK_ALLOW_PRIVATE_TARGETS=true` (dev only). The URL may not contain a fragment (`#…`) or credentials (`https://user@host/`), and may not be an IP literal in a private / loopback / link-local range. |

Your endpoint does **not** need to be reachable at subscription time — the URL is
validated for shape and address range, not by calling it.

---

## 2. Register a subscription

```bash
curl -X POST "$GATEKEEPER/tenants/$TENANT_ID/webhooks" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
        "name": "billing-service user sync",
        "url": "https://billing.example.com/webhooks/gatekeeper",
        "event_types": ["user.created", "user.updated", "user.deleted"],
        "is_active": true
      }'
```

`201 Created`:

```json
{
  "id": "0193c8f2-...",
  "name": "billing-service user sync",
  "url": "https://billing.example.com/webhooks/gatekeeper",
  "eventTypes": ["user.created", "user.updated", "user.deleted"],
  "isActive": true,
  "createdAt": "2026-08-21T10:00:00+00:00",
  "updatedAt": null,
  "secret": "ONLY-TIME-YOU-EVER-SEE-THIS"
}
```

> **Copy `secret` now.** It is stored AES-256-GCM encrypted and is **unrecoverable**
> afterwards — same contract as an OAuth client secret. Losing it means rotating it
> (§8). Put it in your service's secret store, never in source control.

Fields:

- **`event_types`** — omit it or send `[]` to receive **every** event type, including
  ones added later. Any unknown value is a `400` (a typo like `user.create` fails loudly
  instead of producing a subscription that silently never fires).
- **`is_active`** — `false` means the fan-out skips this subscription. Fan-out happens
  **once**, when the event is first published, so re-activating later does **not**
  backfill the events that passed while it was off. Create the subscription active, or
  reconcile afterwards (§9).

Other endpoints (all admin-only, all tenant-scoped):

| Method | Path | Notes |
|---|---|---|
| `GET` | `/tenants/{tenant_id}/webhooks` | Paged (`?page=1&page_size=10`); never returns the secret |
| `GET` | `/tenants/{tenant_id}/webhooks/{id}` | |
| `PUT` | `/tenants/{tenant_id}/webhooks/{id}` | Full replace: `name`, `url`, `event_types`, `is_active`, optional `rotate_secret` |
| `DELETE` | `/tenants/{tenant_id}/webhooks/{id}` | Cascades pending deliveries |
| `POST` | `/tenants/{tenant_id}/webhooks/{id}/test` | Synchronous test delivery (§7) |
| `GET` | `/tenants/{tenant_id}/webhook-deliveries` | The delivery log (§10) |

---

## 3. The wire contract

### Request

```http
POST /webhooks/gatekeeper HTTP/1.1
Content-Type: application/json
User-Agent: Gatekeeper-Webhook/1
X-Gatekeeper-Event-Id: 0193c8f1-1c2a-7d3e-9f04-2b6a5c8d1e7f
X-Gatekeeper-Event-Type: user.created
X-Gatekeeper-Timestamp: 1786060000
X-Gatekeeper-Signature: v1=3f1b0e…
```

| Header | Meaning |
|---|---|
| `X-Gatekeeper-Event-Id` | UUID. **Your idempotency key.** Identical to the body's `id`. |
| `X-Gatekeeper-Event-Type` | `user.created` \| `user.updated` \| `user.deleted`. Same as body `type`. |
| `X-Gatekeeper-Timestamp` | Unix seconds at signing time. Part of the signed material. |
| `X-Gatekeeper-Signature` | `v1=<hex>` — `HMAC-SHA256(secret, "<timestamp>.<raw body>")`. |

Redirects are **not** followed. A `3xx` from your endpoint counts as a failed delivery.

### Envelope

```json
{
  "id": "0193c8f1-1c2a-7d3e-9f04-2b6a5c8d1e7f",
  "type": "user.created",
  "occurred_at": "2026-08-06T20:00:00+00:00",
  "tenant_id": "0193c8f0-...",
  "data": { }
}
```

`id` and `occurred_at` come from the outbox row itself, so the body `id` and the
`X-Gatekeeper-Event-Id` header can never disagree. `occurred_at` is RFC 3339 and is
the time the change **happened**, not the time it was delivered — a retry six hours
later still carries the original timestamp.

### `data` per event type

| Field | Type | `user.created` | `user.updated` | `user.deleted` |
|---|---|---|---|---|
| `user_id` | UUID string | ✅ | ✅ | ✅ |
| `email` | string | ✅ | ✅ | — |
| `email_verified` | bool | ✅ | ✅ | — |
| `display_name` | string \| null | ✅ | ✅ | — |
| `first_name` | string \| null | ✅ | ✅ | — |
| `last_name` | string \| null | ✅ | ✅ | — |
| `is_active` | bool | ✅ | ✅ | — |
| `source` | string \| null | ✅ | `null` | — |
| `created_at` | RFC 3339 | ✅ | ✅ | — |
| `updated_at` | RFC 3339 \| null | ✅ | ✅ | — |

`source` is `self_registration` \| `admin` \| `external_provider` — how the user came
to exist, so you can treat an admin-provisioned account differently from a self
sign-up. It is `null` on `user.updated`.

A `user.deleted` body carries **only** `data.user_id`: there is no user left to
describe, and promising more would be a lie by the time a retry goes out hours later.

### When each event fires

| Type | Emitted when |
|---|---|
| `user.created` | Self-registration, admin creation, or a first social sign-in that **provisions** a new account |
| `user.updated` | Profile/name edit, custom-field edit, or email confirmation flipping `email_verified` to `true` |
| `user.deleted` | An admin deletes a tenant user |

A social sign-in that **links** a provider to an account the tenant already had emits
nothing — no user was created, and a `user.created` there would make every subscriber
insert a duplicate.

### Test deliveries

A test delivery (§7) is byte-identical in shape but adds `"test": true` at the top
level and uses `user_id` `00000000-0000-0000-0000-000000000000` with the email
`webhook-test@example.invalid`. **Check for `test` before writing to your user table.**

---

## 4. The receiver algorithm (language-agnostic)

Nine steps. The order matters — the first four are security, the rest are correctness.

```
on POST /webhooks/gatekeeper:

  1. raw ← the EXACT bytes of the request body      # never a re-serialized object
  2. ts  ← header X-Gatekeeper-Timestamp
     sig ← header X-Gatekeeper-Signature, strip leading "v1="
     if either is missing            → 400, stop

  3. if abs(now_unix() - int(ts)) > 300             # ±5 minutes
        → 400, stop                                 # bounds replay

  4. expected ← hex(HMAC_SHA256(key = secret,
                                msg = utf8(ts) + "." + raw))
     if not constant_time_equal(expected, sig)
        → 401, stop                                 # NEVER use ==

  5. event ← json_parse(raw)                        # only now is the body trusted

  6. if event.tenant_id is not a tenant you mirror  → 202, stop
     if event.test is true                          → 202, stop
     if event.type is unknown to you                → 202, stop   # forward compatible

  7. if already_processed(event.id)                 → 200, stop   # dedupe
     record_seen(event.id)                          # same tx as step 8, or before it

  8. enqueue(event)  — or apply it inline if that is fast

  9. return 200 within a few seconds
```

Notes that are not optional:

- **Step 1 is where most integrations break.** Frameworks that auto-parse JSON hand you
  an object; re-serializing it changes whitespace and key order, and the HMAC will never
  match. Reach for the raw body (§5 shows how per framework).
- **Step 3 without step 4 is useless, and step 4 without step 3 leaves the signature
  valid forever.** They only bound replay together.
- **Step 4 must be constant-time.** A byte-by-byte `==` leaks how much of the signature
  you matched, which is enough to forge one.
- **Step 9:** the request times out after `WEBHOOK_TIMEOUT_SECONDS` (default 10). A slow
  handler earns retries it does not want. Acknowledge, then process.

Any `2xx` is success; `2xx` bodies are ignored. Every non-`2xx` — including `3xx` — is a
failure that schedules a retry.

---

## 5. Reference implementations

The same nine steps in five languages. Each shows the raw-body access that matters;
the dedupe/persist calls are yours.

### Node.js — Express

```js
import express from "express";
import crypto from "node:crypto";

const app = express();
const SECRET = process.env.GATEKEEPER_WEBHOOK_SECRET;

// express.raw, NOT express.json — the signature covers the bytes on the wire.
app.post("/webhooks/gatekeeper",
  express.raw({ type: "application/json" }),
  async (req, res) => {
    const ts  = req.get("X-Gatekeeper-Timestamp");
    const sig = (req.get("X-Gatekeeper-Signature") ?? "").replace(/^v1=/, "");
    if (!ts || !sig) return res.sendStatus(400);

    if (Math.abs(Date.now() / 1000 - Number(ts)) > 300) return res.sendStatus(400);

    const expected = crypto
      .createHmac("sha256", SECRET)
      .update(ts).update(".").update(req.body)   // req.body is a Buffer here
      .digest("hex");

    // timingSafeEqual throws on length mismatch — check length first.
    const a = Buffer.from(expected, "utf8");
    const b = Buffer.from(sig, "utf8");
    if (a.length !== b.length || !crypto.timingSafeEqual(a, b)) {
      return res.sendStatus(401);
    }

    const event = JSON.parse(req.body.toString("utf8"));
    if (event.test) return res.sendStatus(202);

    if (await alreadyProcessed(event.id)) return res.sendStatus(200);
    await enqueue(event);              // fast: a queue insert, not the projection
    res.sendStatus(200);
  });

app.listen(3000);
```

> If a global `app.use(express.json())` runs before this route, the raw body is already
> gone. Either mount `express.raw` first for this path, or capture the raw bytes with
> `express.json({ verify: (req, _res, buf) => { req.rawBody = buf; } })`.

### Python — FastAPI

```python
import hashlib, hmac, json, time
from fastapi import FastAPI, Request, Response

app = FastAPI()
SECRET = os.environ["GATEKEEPER_WEBHOOK_SECRET"].encode()

@app.post("/webhooks/gatekeeper")
async def receive(request: Request):
    raw = await request.body()                     # exact bytes
    ts  = request.headers.get("X-Gatekeeper-Timestamp")
    sig = (request.headers.get("X-Gatekeeper-Signature") or "").removeprefix("v1=")
    if not ts or not sig:
        return Response(status_code=400)

    if abs(time.time() - int(ts)) > 300:
        return Response(status_code=400)

    expected = hmac.new(SECRET, ts.encode() + b"." + raw, hashlib.sha256).hexdigest()
    if not hmac.compare_digest(expected, sig):     # compare_digest, never ==
        return Response(status_code=401)

    event = json.loads(raw)
    if event.get("test"):
        return Response(status_code=202)

    if await already_processed(event["id"]):
        return Response(status_code=200)
    await enqueue(event)
    return Response(status_code=200)
```

Flask equivalent: `raw = request.get_data()` (never `request.json` first).
Django: `raw = request.body`.

### Go — net/http

```go
func handler(w http.ResponseWriter, r *http.Request) {
    raw, err := io.ReadAll(io.LimitReader(r.Body, 1<<20)) // cap the read
    if err != nil {
        w.WriteHeader(http.StatusBadRequest)
        return
    }

    ts := r.Header.Get("X-Gatekeeper-Timestamp")
    sig := strings.TrimPrefix(r.Header.Get("X-Gatekeeper-Signature"), "v1=")
    sent, err := strconv.ParseInt(ts, 10, 64)
    if ts == "" || sig == "" || err != nil {
        w.WriteHeader(http.StatusBadRequest)
        return
    }

    if math.Abs(float64(time.Now().Unix()-sent)) > 300 {
        w.WriteHeader(http.StatusBadRequest)
        return
    }

    mac := hmac.New(sha256.New, []byte(secret))
    mac.Write([]byte(ts))
    mac.Write([]byte("."))
    mac.Write(raw)
    expected := hex.EncodeToString(mac.Sum(nil))

    if !hmac.Equal([]byte(expected), []byte(sig)) { // constant time
        w.WriteHeader(http.StatusUnauthorized)
        return
    }

    var event Event
    if err := json.Unmarshal(raw, &event); err != nil {
        w.WriteHeader(http.StatusBadRequest)
        return
    }
    if event.Test {
        w.WriteHeader(http.StatusAccepted)
        return
    }

    if seen, _ := alreadyProcessed(event.ID); seen {
        w.WriteHeader(http.StatusOK)
        return
    }
    _ = enqueue(event)
    w.WriteHeader(http.StatusOK)
}
```

### Java — Spring Boot

```java
@PostMapping(path = "/webhooks/gatekeeper", consumes = MediaType.APPLICATION_JSON_VALUE)
public ResponseEntity<Void> receive(
        @RequestBody byte[] raw,                       // byte[], not a DTO
        @RequestHeader("X-Gatekeeper-Timestamp") String ts,
        @RequestHeader("X-Gatekeeper-Signature") String signatureHeader) throws Exception {

    String sig = signatureHeader.startsWith("v1=") ? signatureHeader.substring(3) : signatureHeader;

    if (Math.abs(Instant.now().getEpochSecond() - Long.parseLong(ts)) > 300) {
        return ResponseEntity.badRequest().build();
    }

    Mac mac = Mac.getInstance("HmacSHA256");
    mac.init(new SecretKeySpec(secret.getBytes(UTF_8), "HmacSHA256"));
    mac.update(ts.getBytes(UTF_8));
    mac.update((byte) '.');
    mac.update(raw);
    String expected = HexFormat.of().formatHex(mac.doFinal());

    // MessageDigest.isEqual is constant-time on modern JDKs.
    if (!MessageDigest.isEqual(expected.getBytes(UTF_8), sig.getBytes(UTF_8))) {
        return ResponseEntity.status(HttpStatus.UNAUTHORIZED).build();
    }

    Event event = objectMapper.readValue(raw, Event.class);
    if (event.test()) return ResponseEntity.accepted().build();

    if (store.alreadyProcessed(event.id())) return ResponseEntity.ok().build();
    queue.enqueue(event);
    return ResponseEntity.ok().build();
}
```

### PHP

```php
<?php
$raw = file_get_contents('php://input');           // exact bytes
$ts  = $_SERVER['HTTP_X_GATEKEEPER_TIMESTAMP'] ?? '';
$sig = preg_replace('/^v1=/', '', $_SERVER['HTTP_X_GATEKEEPER_SIGNATURE'] ?? '');

if ($ts === '' || $sig === '' || abs(time() - (int) $ts) > 300) {
    http_response_code(400);
    exit;
}

$expected = hash_hmac('sha256', $ts . '.' . $raw, getenv('GATEKEEPER_WEBHOOK_SECRET'));
if (!hash_equals($expected, $sig)) {               // hash_equals, never ===
    http_response_code(401);
    exit;
}

$event = json_decode($raw, true);
if (!empty($event['test'])) { http_response_code(202); exit; }

if (already_processed($event['id'])) { http_response_code(200); exit; }
enqueue($event);
http_response_code(200);
```

### Any other language

You need exactly four primitives — see [Appendix B](#appendix-b--primitives-per-language):
raw body access, HMAC-SHA256, hex encoding, constant-time comparison. Validate your
implementation against the [test vectors](#appendix-a--hmac-test-vectors) before
pointing a subscription at it.

---

## 6. Projecting events into your own database

**Delivery is at-least-once and unordered.** Those are the two properties your
projection has to survive.

### Dedupe table

```sql
CREATE TABLE gatekeeper_processed_events (
    event_id     uuid PRIMARY KEY,
    event_type   text        NOT NULL,
    occurred_at  timestamptz NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now()
);
```

Insert with `ON CONFLICT DO NOTHING` **in the same transaction** as the projection. If
the insert affects zero rows, you have seen this event — commit and return `200`. This
is what makes a duplicate delivery free instead of a duplicated row.

Prune rows older than a week or two: retries stop after ~7 hours, so nothing older can
ever arrive again.

### Idempotent upsert

```sql
INSERT INTO users (id, email, email_verified, display_name, first_name, last_name,
                   is_active, source, gatekeeper_updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (id) DO UPDATE SET
    email                 = EXCLUDED.email,
    email_verified        = EXCLUDED.email_verified,
    display_name          = EXCLUDED.display_name,
    first_name            = EXCLUDED.first_name,
    last_name             = EXCLUDED.last_name,
    is_active             = EXCLUDED.is_active,
    gatekeeper_updated_at = EXCLUDED.gatekeeper_updated_at
-- Drop a stale event that arrived after a newer one (retry overtaken by a later change).
WHERE users.gatekeeper_updated_at < EXCLUDED.gatekeeper_updated_at;
```

Handle `user.created` and `user.updated` with the **same** upsert. Treat every event as
the full current state of the user, not a delta — that is what makes ordering
irrelevant. Use `updated_at` (falling back to `created_at`, or to the envelope's
`occurred_at`) as the freshness stamp.

Do not assume `user.created` arrives before `user.updated` for the same user: a
`user.created` whose first attempt failed can land after an update that succeeded on
its first try.

### Deletion

`user.deleted` gives you only `user_id`. Decide deliberately:

- **Soft-delete / deactivate** (recommended) if your rows carry history, invoices, or
  audit trails that must outlive the identity.
- **Hard delete** if you are mirroring identity only — and remember the erasure request
  that triggered this is a GDPR/LGPD obligation, so it must actually propagate.

A `user.deleted` for a user you never saw is normal (you subscribed after they were
created). Make it a no-op, not an error.

---

## 7. Testing end to end

**1. Fire a test delivery.** Synchronous, bypasses the outbox and the retry machinery,
and tells you exactly what your endpoint returned:

```bash
curl -X POST "$GATEKEEPER/tenants/$TENANT_ID/webhooks/$SUB_ID/test" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

```json
{ "success": true, "status": 200, "error": null }
```

`success: false` with `status: null` means the request never completed — DNS failure,
TLS error, connection refused, or timeout. With a status, that status is what your
endpoint answered.

The test body carries `"test": true`, a nil `user_id`, and
`webhook-test@example.invalid`, so an endpoint that wires it straight into a user table
cannot mistake it for a real person — but see step 6 of the algorithm and skip it
explicitly.

**2. Verify your signature check actually rejects.** Replay the same request with one
byte of the body changed and confirm you return `401`. An integration that accepts
everything passes step 1 just as happily as a correct one.

**3. Local development.** The URL validator refuses private and loopback targets, so
either:

- point the subscription at a public tunnel (`ngrok`, `cloudflared`, …) — the realistic
  option, and it exercises TLS; or
- set `WEBHOOK_ALLOW_PRIVATE_TARGETS=true` on the Gatekeeper instance, which permits
  `http://localhost:3000/...`. **Development and self-hosted only** — enabling it on a
  cloud instance lets a tenant admin point a webhook at the instance metadata endpoint.

**4. Trigger a real event.** Register a user (or create one via
`POST /tenants/{tenant_id}/users`) and watch for `user.created`. Expect it within about
one poll interval (default 5s).

---

## 8. Rotating the secret without downtime

Rotation invalidates the old secret **immediately** — including for deliveries already
queued for retry. So make the consumer accept both before you rotate:

1. **Deploy first.** Have your verifier accept a *list* of secrets: compute the HMAC
   against each and pass if any matches (constant-time each time). Ship with the current
   secret only.
2. **Rotate:**

   ```bash
   curl -X PUT "$GATEKEEPER/tenants/$TENANT_ID/webhooks/$SUB_ID" \
     -H "Authorization: Bearer $ADMIN_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"name":"billing-service user sync",
          "url":"https://billing.example.com/webhooks/gatekeeper",
          "event_types":["user.created","user.updated","user.deleted"],
          "is_active":true,
          "rotate_secret":true}'
   ```

   `PUT` is a **full replace** — send every field, not just the ones you are changing.
   The response carries `secret` only when `rotate_secret` was `true`; that is your one
   chance to copy it.
3. **Add the new secret** to the consumer's list and deploy.
4. **Remove the old secret** after the retry window (~7 hours) has passed, so no
   in-flight retry signed with it is still arriving.

An ordinary edit (rename, URL change, toggling `is_active`) does **not** touch the
secret.

---

## 9. Initial backfill and reconciliation

**Webhooks only start at subscription time.** Users that already existed produce no
events, and a subscription that was `is_active: false` for a while does not backfill
when re-enabled.

For the initial sync, page the admin user list and run it through the *same* upsert as
the webhook path:

```bash
curl "$GATEKEEPER/tenants/$TENANT_ID/users?page=1&page_size=100" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

Responses are `{ "items": [...], "total": n, "page": n, "page_size": n }`.

Order of operations matters: **create the subscription first, then backfill.** Events
that fire during the backfill are then either delivered (and idempotently re-applied) or
already covered by the page you fetched. Backfilling first leaves a gap between the last
page and the subscription going live.

Re-run the same backfill periodically (nightly is plenty) as reconciliation — it repairs
anything a `dead` delivery lost. Because every write is an idempotent upsert keyed on
freshness, a reconciliation pass is a no-op when nothing drifted.

---

## 10. Troubleshooting

Start at the delivery log — it is the authoritative answer to "did you even try?":

```bash
curl "$GATEKEEPER/tenants/$TENANT_ID/webhook-deliveries?page=1&page_size=20" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

Each row: `id`, `eventId`, `subscriptionId`, `status` (`pending` \| `delivered` \|
`dead`), `attempts`, `nextAttemptAt`, `lastStatus`, `lastError`, `createdAt`,
`deliveredAt`. The payload is deliberately **not** in this projection — it holds user
PII and this is a diagnostics surface.

| Symptom | Cause | Fix |
|---|---|---|
| Signature never matches | Body was parsed and re-serialized before hashing | Hash the raw bytes (§4 step 1) |
| Signature never matches, raw body is correct | Signed the body alone, or used a separator other than `.` | Signed material is `"<timestamp>.<raw body>"` |
| Works in tests, fails in production | Proxy or gateway rewriting the body (recompression, charset, JSON normalization) | Make the proxy pass the body through untouched |
| Nothing arrives, no delivery rows | Subscription `isActive: false`, wrong tenant, or `event_types` filters the type out | `GET /tenants/{id}/webhooks`; empty `event_types` = all |
| No delivery rows for events you know happened | Subscription created *after* the event, or dispatcher off (`WEBHOOKS_ENABLED=false`) | Backfill (§9); check the operator's config |
| `lastStatus: null`, `lastError` set | Transport failure — DNS, TLS, connection refused, timeout | Check the endpoint is publicly resolvable and answers within 10s |
| `lastStatus: 301/302` | Redirect. Redirects are deliberately not followed | Subscribe the final URL directly |
| `status: dead` | 6 attempts failed over ~7 hours | Fix the endpoint, then reconcile (§9) — dead deliveries are not retried |
| Duplicate rows in your DB | No dedupe on `id` | §6 — at-least-once is the contract, not a bug |
| A "user" named Webhook Test appears | Test delivery projected as real | Skip `"test": true` (§4 step 6) |
| Events for the wrong tenant | One endpoint serving several subscriptions | Check `tenant_id` in the envelope; prefer one endpoint per tenant |

### Retry schedule

| Attempt | Delay after previous |
|---|---|
| 1 | immediate |
| 2 | 30s |
| 3 | 2m |
| 4 | 10m |
| 5 | 1h |
| 6 | 6h |

Then `dead`, kept for inspection. Roughly a 7-hour window — a consumer having a bad
afternoon still gets its events.

---

## 11. Go-live checklist

- [ ] Endpoint is HTTPS with a valid certificate chain.
- [ ] Secret lives in a secret store, injected as config — not in source control.
- [ ] Verification uses the **raw** body, hashes `"<ts>.<body>"`, compares in constant time.
- [ ] Timestamp skew rejected beyond ±5 minutes.
- [ ] Signature failures return `401` and are logged with the event id.
- [ ] `test: true` deliveries are skipped before any write.
- [ ] Unknown event types return `2xx` and are ignored (forward compatible).
- [ ] Dedupe on `id` in the same transaction as the projection.
- [ ] Projection is an idempotent upsert guarded by a freshness stamp.
- [ ] Handler acknowledges in well under 10s; heavy work is async.
- [ ] `user.deleted` for an unknown user is a no-op.
- [ ] Consumer can verify against **more than one** secret (makes rotation a non-event).
- [ ] Initial backfill run *after* the subscription went live, plus a periodic reconcile.
- [ ] Alerting on delivery failures — your own 5xx rate on this route, and periodic checks of
      `webhook-deliveries` for `dead` rows.

---

## Appendix A — HMAC test vectors

Pinned by the producer's own unit tests. Any correct implementation reproduces these
exactly:

| secret | timestamp | body | signature (hex) |
|---|---|---|---|
| `key` | `1` | `body` | `91b5374b153842ad05b2c4eab9349b8321b14703165bd3fb8b034dfb8be98ae5` |
| `s3cret` | `1786060000` | `{"id":"x"}` | `f07fd5bf36f055ca3b2ff2338ec54f3dd974c81c55c3bfaad7477d36793947f0` |

Checkable from a shell, with no code at all:

```bash
printf '1.body' | openssl dgst -sha256 -hmac key
# SHA2-256(stdin)= 91b5374b153842ad05b2c4eab9349b8321b14703165bd3fb8b034dfb8be98ae5

printf '1786060000.{"id":"x"}' | openssl dgst -sha256 -hmac s3cret
# SHA2-256(stdin)= f07fd5bf36f055ca3b2ff2338ec54f3dd974c81c55c3bfaad7477d36793947f0
```

The header value is this hex string prefixed with `v1=`.

---

## Appendix B — primitives per language

| Language | Raw body | HMAC-SHA256 | Constant-time compare |
|---|---|---|---|
| Node.js | `express.raw()` / `req.rawBody` | `crypto.createHmac("sha256", k)` | `crypto.timingSafeEqual` (check lengths first) |
| Python | `await request.body()` / `request.get_data()` | `hmac.new(k, m, hashlib.sha256)` | `hmac.compare_digest` |
| Go | `io.ReadAll(r.Body)` | `hmac.New(sha256.New, k)` | `hmac.Equal` |
| Java | `@RequestBody byte[]` | `Mac.getInstance("HmacSHA256")` | `MessageDigest.isEqual` |
| PHP | `file_get_contents('php://input')` | `hash_hmac('sha256', …)` | `hash_equals` |
| Ruby | `request.body.read` | `OpenSSL::HMAC.hexdigest("SHA256", …)` | `Rack::Utils.secure_compare` |
| C# / .NET | `EnableBuffering` + read stream | `new HMACSHA256(key)` | `CryptographicOperations.FixedTimeEquals` |
| Rust | `axum::body::Bytes` extractor | `hmac::Hmac<Sha256>` | `subtle::ConstantTimeEq` |

Common trap in every row of the first column: a framework that has already parsed the
body has already destroyed your ability to verify it. Capture the bytes first.
