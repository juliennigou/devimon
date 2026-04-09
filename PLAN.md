# Devimon — Implementation Plan

## Tech Stack

- **Language**: Rust
- **CLI parsing**: `clap` (subcommands: status, feed, play, rest)
- **File watching**: `notify` crate (detect file modifications)
- **Persistence**: JSON file (`~/.devimon/save.json`) via `serde_json`
- **Terminal rendering**: `colored` for colored output (ratatui deferred to V2)
- **Time tracking**: `chrono`

---

## Architecture Overview

```
devimon/
├── src/
│   ├── main.rs          # CLI entry point, command dispatch
│   ├── monster.rs       # Monster struct, needs, XP, evolution
│   ├── watcher.rs       # File system watcher, XP events
│   ├── session.rs       # Session tracking (active time)
│   ├── save.rs          # Load/save state from disk
│   ├── display.rs       # Terminal rendering (ASCII art + status)
│   └── xp.rs            # XP rules, anti-spam, caps
├── Cargo.toml
├── devimon.md
└── PLAN.md
```

---

## Data Model

### Monster

```rust
struct Monster {
    name: String,
    level: u32,
    xp: u32,
    xp_to_next: u32,
    stage: Stage,        // Baby | Young | Evolved

    // Needs (0.0 - 100.0)
    hunger: f32,         // decreases over time
    energy: f32,         // decreases over time
    mood: f32,           // affected by neglect and activity

    last_fed: DateTime,
    last_played: DateTime,
    last_rested: DateTime,
    last_active: DateTime,
    created_at: DateTime,
}

enum Stage { Baby, Young, Evolved }
```

### XP State (anti-spam)

```rust
struct XpState {
    xp_this_minute: u32,
    minute_bucket: DateTime,   // which minute we're tracking
    files_modified_today: u32,
}
```

---

## Core Systems

### 1. XP System

| Event | XP | Cap |
|---|---|---|
| File modified | +1 XP | 10 XP/minute max |
| Multiple files in burst (3+) | +2 bonus | included in cap |
| Consistent activity (>5 min) | +1 bonus/5min | — |

Rules:
- XP cap: 10/minute (anti-spam)
- No XP awarded if monster energy < 10 (too tired)
- Bonus multiplier when all needs > 70

### 2. Needs Decay

Needs decrease passively over real time (not play time):

| Need | Decay rate |
|---|---|
| Hunger | -5 / hour |
| Energy | -3 / hour |
| Mood | -2 / hour (more if hunger < 20) |

Mood also increases slightly during active work sessions (+1 / 10min active).

### 3. Player Actions

| Command | Effect | Cooldown |
|---|---|---|
| `pet feed` | Hunger +40, Mood +5 | 2h |
| `pet play` | Mood +30, Energy -10 | 1h |
| `pet rest` | Energy +50, Mood +5 | 4h |

### 4. Evolution

| Stage | Requirement |
|---|---|
| Baby → Young | Level 5, all needs ever > 50 |
| Young → Evolved | Level 15, mood avg > 60 over last 7 days |

### 5. File Watcher (background daemon)

- Runs as a background process (`devimon watch`)
- Watches current directory recursively
- Writes XP events to `~/.devimon/events.json` (append-only queue)
- Main CLI drains events on next invocation

> This avoids a persistent daemon — events are buffered and applied lazily when the user runs any `pet` command.

---

## ASCII Art (per stage)

```
Baby:          Young:         Evolved:
  (o_o)          (^o^)          (>O<)
  /||\           /|||\          /||||\
  d  b           d   b          d    b
```

Mood variants (shown in `pet status`):
- Happy: `(^o^)`
- Neutral: `(-_-)`
- Sad: `(;_;)`

---

## CLI Commands

```
pet spawn [name]     Create a new monster
pet status           Show monster state (needs, XP, level, ASCII)
pet feed             Feed the monster
pet play             Play with the monster
pet rest             Put the monster to rest
pet watch            Start file watcher in background
```

---

## Save Format (`~/.devimon/save.json`)

```json
{
  "monster": { ... },
  "xp_state": { ... },
  "version": 1
}
```

Events queue (`~/.devimon/events.json`):
```json
[
  { "type": "file_modified", "path": "src/main.rs", "timestamp": "..." },
  ...
]
```

---

## Implementation Phases

### Phase 1 — Core Foundation
- [ ] `Cargo.toml` setup (clap, serde, chrono, colored)
- [ ] `save.rs`: load/save monster state from `~/.devimon/`
- [ ] `monster.rs`: Monster struct, serialization
- [ ] `pet spawn` command working

### Phase 2 — Needs & Actions
- [ ] Passive decay logic (computed on load based on elapsed time)
- [ ] `pet feed`, `pet play`, `pet rest` with cooldowns
- [ ] `pet status` with ASCII art and colored bars

### Phase 3 — XP & File Watching
- [ ] `xp.rs`: XP rules, per-minute cap, anti-spam
- [ ] `watcher.rs`: file watcher using `notify`, writes to events queue
- [ ] `pet watch` command (background via `nohup` or similar)
- [ ] Event draining on CLI invocation

### Phase 4 — Progression
- [ ] Level-up logic (XP thresholds per level)
- [ ] Stage evolution checks
- [ ] Mood/personality messages based on state

### Phase 5 — Polish
- [ ] Onboarding message on first spawn
- [ ] Monster reactions ("Session productive!", "Tu sembles fatigué…")
- [ ] Error handling and edge cases (no save file, corrupted state)
- [ ] README with setup instructions

---

## Key Design Decisions

1. **No persistent daemon** — events are buffered to disk, applied lazily. Simple and reliable.
2. **Time-based decay** — computed from `last_active` timestamps on each load. No background process needed for needs.
3. **No punishment** — if needs hit 0, XP gain slows but nothing is lost. Philosophy: reward regularity, not punish neglect.
4. **XP cap prevents cheating** — touching a file 1000 times in a minute gives the same XP as touching it 10 times.
