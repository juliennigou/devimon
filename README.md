# Devimon

Devimon is a terminal monster that grows from your real file activity.

It now includes:

- local terminal gameplay
- GitHub login from the CLI
- cloud sync for your monster
- a public leaderboard website

## Install

### Fast install from GitHub

If Rust is already installed:

```bash
cargo install --git https://github.com/juliennigou/devimon --locked
```

This installs the global `devimon` command.

### One-command installer

```bash
curl -fsSL https://raw.githubusercontent.com/juliennigou/devimon/main/install.sh | bash
```

This installer:

- checks that `cargo` exists
- installs Devimon from the GitHub repo
- tells you where the binary is installed

## Update

To update on any machine:

```bash
cargo install --git https://github.com/juliennigou/devimon --locked --force
```

## Usage

Start the TUI:

```bash
devimon
```

Useful commands:

```bash
devimon spawn Devi
devimon status
devimon feed
devimon play
devimon rest
devimon login
devimon whoami
devimon sync
```

## How it works

- Devimon watches the directory where you run it
- file activity is converted into XP
- your monster levels up locally
- if you log in, the monster can sync online
- the synced monster appears on the leaderboard

## Cloud leaderboard

Production URLs:

- API: `https://devimon-api.julienigou33.workers.dev`
- Leaderboard: `https://leaderboard.devimon-leaderboard.pages.dev`

After installing, you can join the leaderboard with:

```bash
devimon login
devimon sync
```

## CI/CD

GitHub Actions now handles:

- CI on pushes and pull requests
- automatic deploys from `main`

Workflows:

- `.github/workflows/ci.yml`
- `.github/workflows/deploy.yml`

### Required GitHub repository secrets

Set these in the GitHub repository settings before the deploy workflow can work:

- `CLOUDFLARE_API_TOKEN`
- `CLOUDFLARE_ACCOUNT_ID`

The Worker's GitHub OAuth secrets are still managed in Cloudflare itself:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`

Those two do not need to live in GitHub Actions if they are already stored in the deployed Worker.

### Deploy behavior

On every push to `main`, GitHub Actions will:

1. run `cargo check`
2. install the worker dependencies
3. apply `cloudflare/worker/schema.sql` to the remote D1 database
4. deploy the Cloudflare Worker
5. deploy the Pages website

If you change `main`, production should redeploy automatically once the workflow secrets are set.

## Local development

Run the app locally:

```bash
cargo run
```

Run the cloud worker locally:

```bash
cd cloudflare/worker
npm install
npx wrangler dev --local
```

Run the website locally:

```bash
cd cloudflare/site
python3 -m http.server 4173
```

More cloud setup details are in [cloudflare/README.md](cloudflare/README.md).
