# Devimon

```
   .-^-.
 .( ^o^ ).    Devimon — your terminal companion.
  /|___|\ 
  d_/ \_b     Raise a monster. Let it grow while you code.
```

Devimon is a terminal-native virtual pet that lives in your shell. It watches your file activity, levels up from your coding sessions, and can sync to a global leaderboard to compete with other developers.

---

## Install

### Option 1 — Download a pre-built binary (no Rust required)

Go to [Releases](https://github.com/juliennigou/devimon/releases/latest) and download the binary for your platform:

| Platform        | File                       |
|-----------------|----------------------------|
| macOS ARM64 (M1/M2/M3) | `devimon-macos-arm64`  |
| macOS Intel     | `devimon-macos-x86_64`     |
| Linux x86_64    | `devimon-linux-x86_64`     |
| Linux ARM64     | `devimon-linux-arm64`      |

Then install it:

```bash
# Example for macOS ARM64
curl -L https://github.com/juliennigou/devimon/releases/latest/download/devimon-macos-arm64 -o devimon
chmod +x devimon
sudo mv devimon /usr/local/bin/
```

### Option 2 — One-line installer (requires Rust + cargo)

```bash
curl -fsSL https://raw.githubusercontent.com/juliennigou/devimon/main/install.sh | bash
```

This checks that `cargo` is installed, then builds and installs Devimon from source.

### Option 3 — From source with cargo (requires Rust)

```bash
cargo install --git https://github.com/juliennigou/devimon --locked
```

> **Note:** Devimon is not published on crates.io. `cargo install devimon` will not work. Use the git URL above.

### Verify

```bash
devimon --help
```

---

## Quick start

```bash
# 1. Spawn your monster
devimon spawn Devi

# 2. Open the interactive TUI
devimon

# 3. Start the file watcher in your project directory
#    (run this in a separate terminal while you code)
devimon watch
```

---

## How it works

Devimon watches the directory where you run `devimon watch` for file changes. Every file modification earns XP:

- **1 XP** per file modified per minute
- **+2 bonus XP** if you modify 3 or more files in the same minute (burst)
- **×1.25 multiplier** when all stats (Hunger, Energy, Mood) are above 70%
- **0 XP** if Energy drops below 10% — rest your monster first
- Capped at **10 XP per minute**

Your monster has three stats that decay over time if you ignore them:

| Stat    | Action to restore | Cooldown |
|---------|-------------------|----------|
| Hunger  | `devimon feed`    | 2 hours  |
| Energy  | `devimon rest`    | 4 hours  |
| Mood    | `devimon play`    | 1 hour   |

---

## Evolution

Monsters evolve through three stages as they accumulate XP and hit stat milestones:

```
Baby  ──►  Young  ──►  Evolved
```

Each stage unlocks new ASCII art and animations in the TUI. There are two species:

- **Devimon** (default) — the classic terminal demon
- **Dragon** — unlockable via `devimon spawn <name> --species dragon`

---

## Commands

```
devimon                   Launch the interactive TUI
devimon spawn <name>      Spawn a new monster
  --species <dragon>      Choose species (default: devimon)
devimon status            Print current stats
devimon feed              Feed your monster (+40 Hunger, +5 Mood)
devimon play              Play with your monster (+30 Mood, -10 Energy)
devimon rest              Let it rest (+50 Energy, +5 Mood)
devimon watch             Start the file watcher in the current directory
devimon login             Link your monster to a GitHub account
devimon logout            Clear the local session
devimon whoami            Show the connected account
devimon sync              Upload monster state to the cloud leaderboard
devimon update            Update Devimon to the latest version
```

---

## Cloud leaderboard

Once your monster is ready to compete:

```bash
devimon login    # opens GitHub device flow in your browser
devimon sync     # uploads your monster to the leaderboard
```

The leaderboard is live at:
- **Website:** https://leaderboard.devimon-leaderboard.pages.dev
- **API:** https://devimon-api.julienigou33.workers.dev/api/leaderboard

---

## Local development

**Rust CLI:**

```bash
cargo run
```

**Cloudflare Worker (API):**

```bash
cd cloudflare/worker
npm install
npx wrangler dev --local
```

**Leaderboard website:**

```bash
cd cloudflare/site
python3 -m http.server 4173
# → http://localhost:4173
```

The Rust client automatically points to `http://127.0.0.1:8787` when run on localhost.

---

## CI/CD

GitHub Actions handles everything on push to `main`:

1. `cargo check` + formatting check
2. Worker dependency install + syntax check
3. D1 schema migration (`cloudflare/worker/schema.sql`)
4. Cloudflare Worker deploy
5. Cloudflare Pages deploy

**Required GitHub repository secrets:**

| Secret | Purpose |
|--------|---------|
| `CLOUDFLARE_API_TOKEN` | Deploy to Cloudflare |
| `CLOUDFLARE_ACCOUNT_ID` | Target account |

The GitHub OAuth secrets (`GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`) live in the Cloudflare Worker environment directly and do not need to be in GitHub Actions.

**Releases** are triggered by pushing a version tag:

```bash
git tag v0.1.3
git push origin v0.1.3
```

This builds binaries for macOS ARM64/x86_64, Linux x86_64/ARM64 and publishes them as a GitHub Release.

---

## Project structure

```
devimon/
├── src/               Rust CLI + TUI
│   ├── main.rs        Commands (clap)
│   ├── ui.rs          Ratatui TUI
│   ├── display.rs     ASCII art & animations
│   ├── monster.rs     Monster model & evolution
│   ├── actions.rs     Feed / play / rest logic
│   ├── xp.rs          File-event → XP engine
│   ├── cloud.rs       GitHub OAuth + sync
│   └── watcher.rs     File system watcher
├── cloudflare/
│   ├── worker/        Cloudflare Worker (Node.js + D1)
│   └── site/          Static leaderboard website
├── install.sh         One-line installer
└── Cargo.toml
```
