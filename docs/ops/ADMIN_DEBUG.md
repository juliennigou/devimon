# Admin Debug Access

This note documents the lightweight worker admin/debug flow for suspicious ranked sync inspection.

## Purpose

Use the admin/debug endpoint to inspect recent suspicious ranked syncs recorded by the Worker.

Current endpoint:

- `GET /api/admin/suspicious-syncs`

## Required Secret

Set this Worker secret before using the endpoint:

- `ADMIN_DEBUG_TOKEN`

Example:

```bash
cd cloudflare/worker
npx wrangler secret put ADMIN_DEBUG_TOKEN
```

## Authentication

The endpoint accepts either:

- `x-admin-token: <token>`
- `Authorization: Bearer <token>`

## Query Parameters

- `limit`
  Number of rows to return. Clamped to `1..100`.
- `account_id`
  Optional filter for a specific Devimon account.
- `severity`
  Optional filter. Allowed values:
  - `warn`
  - `high`

## Example Requests

Recent suspicious syncs:

```bash
curl \
  -H "x-admin-token: $ADMIN_DEBUG_TOKEN" \
  "https://your-worker.example.com/api/admin/suspicious-syncs"
```

Only high-severity rows:

```bash
curl \
  -H "x-admin-token: $ADMIN_DEBUG_TOKEN" \
  "https://your-worker.example.com/api/admin/suspicious-syncs?severity=high&limit=20"
```

Filter one account:

```bash
curl \
  -H "Authorization: Bearer $ADMIN_DEBUG_TOKEN" \
  "https://your-worker.example.com/api/admin/suspicious-syncs?account_id=<account_id>&limit=50"
```

## Response Shape

The response includes:

- `generated_at`
- `filters`
- `suspicious_syncs`

Each suspicious sync row includes:

- `account_id`
- `monster_id`
- `device_id`
- `reason`
- `severity`
- `requested_ranked_xp_delta`
- `accepted_ranked_xp_delta`
- `max_accepted_ranked_xp_delta`
- `trusted_total_xp_after`
- `detected_at`

## Reason Guide

### `ranked_xp_without_elapsed_time`

Ranked XP evidence arrived even though the server observed no elapsed sync time window.

Typical meaning:

- duplicate or immediate repeated sync
- suspicious manual replay
- malformed client behavior

### `ranked_xp_capped`

The client requested more ranked XP than the server was willing to accept for the elapsed time window.

Typical meaning:

- local backlog larger than trusted allowance
- intentionally inflated ranked evidence
- benign offline accumulation that exceeded the current trust model

### `ranked_xp_implausible_burst`

The requested ranked XP was extremely large relative to the tiny accepted window.

Typical meaning:

- likely manual or scripted inflation attempt
- severe client/reporting bug

## Operator Guidance

- `warn` means reviewable but not automatically hostile.
- `high` means the sync is strongly suspicious and should be prioritized for inspection.
- A capped sync is not necessarily malicious, but repeated high-severity entries for the same account or device should be treated as likely abuse or instrumentation failure.

## Current Limitations

- This is read-only debugging, not an admin moderation system.
- There is no aggregation endpoint yet.
- There is no UI yet; access is currently direct via HTTP.
