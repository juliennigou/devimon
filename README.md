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
