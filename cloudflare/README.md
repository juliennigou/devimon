# Devimon Cloud Setup

This folder contains the hosted pieces for the Devimon leaderboard:

- `worker/`: Cloudflare Worker API + D1 schema
- `site/`: static leaderboard website

## Worker

Files:

- `worker/src/index.js`
- `worker/schema.sql`
- `worker/wrangler.toml`

### Required secrets

Set these in the Worker before deployment:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `ADMIN_DEBUG_TOKEN` for the admin/debug suspicious-sync endpoint

### Initial D1 setup

1. Create a D1 database.
2. Put the generated database ID into `worker/wrangler.toml`.
3. Apply the schema:

```bash
cd cloudflare/worker
npm install
npx wrangler d1 execute devimon --file=./schema.sql
```

`schema.sql` is the safe bootstrap schema. Ranked leaderboard columns and indexes are also
backfilled lazily by the Worker at runtime so older D1 databases can upgrade without
failing on missing-column index creation.

### Run locally

```bash
cd cloudflare/worker
npm install
npx wrangler dev
```

The Rust client defaults to `http://127.0.0.1:8787`, which matches local Worker development.

### Admin/debug endpoint

The Worker exposes a lightweight read-only debug endpoint for suspicious ranked syncs:

- `GET /api/admin/suspicious-syncs`

It is protected by `ADMIN_DEBUG_TOKEN`.

Full operator usage is documented in [docs/ops/ADMIN_DEBUG.md](/Users/juliennigou/devimon/docs/ops/ADMIN_DEBUG.md).

## GitHub OAuth

The terminal app uses GitHub device flow through the Worker:

- `POST /api/auth/github/device/start`
- `POST /api/auth/github/device/poll`

Create a GitHub OAuth app and configure its client ID and secret in the Worker.

## Leaderboard site

The website is a static frontend in `site/`.

You can publish it with Cloudflare Pages or serve it from any static host.

### API base URL

If the site is not hosted on the same domain as the Worker, set:

```html
<script>
  window.DEVIMON_API_BASE_URL = "https://your-worker-url";
</script>
```

before `app.js` in `site/index.html`.
