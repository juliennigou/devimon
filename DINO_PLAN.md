# Dino Rebuild Plan

## Goal

Rebuild the Dino mini-game so it matches the existing Devimon architecture:

- `src/ui.rs` stays the TUI host
- `src/save.rs` owns persistence and rewards
- `src/display.rs` owns monster-specific ASCII poses
- `src/dino/` owns Dino gameplay rules, rendering projection, collision, and input mapping

The target is Chromium-faithful mechanics with terminal-native rendering.

## Current State

Today the Dino implementation is embedded inside `src/ui.rs`.

That makes it hard to:

- evolve physics without touching unrelated TUI code
- test collision and spawn rules in isolation
- add richer states like `Ready`, `Starting`, `Paused`
- expand monster-specific Dino poses cleanly

## Target Module Layout

### `src/dino/domain.rs`

Pure gameplay data:

- `DinoGameSession`
- `DinoGameStatus`
- `DinoObstacle`
- `DinoObstacleKind`
- future `GamePhase`, `Trex`, `Score`, `Config`

### `src/dino/update.rs`

Simulation rules:

- fixed-step update
- jump / gravity / duck / speed drop
- spawn scheduling
- distance and score progression
- difficulty ramp

### `src/dino/collision.rs`

Collision detection:

- broad-phase AABB
- narrow-phase sub-hitboxes
- future debug hitbox support

### `src/dino/render.rs`

Terminal projection:

- logical world -> terminal cells
- ground / obstacle / runner composition
- size adaptation without changing gameplay rules

### `src/dino/input.rs`

Internal commands:

- `JumpPressed`
- `JumpReleased`
- `DuckPressed`
- `DuckReleased`
- `Restart`
- `Quit`

### `src/dino/integration.rs`

Host-facing integration:

- crash/result payloads
- exit reasons
- bridge between the Dino module and `ui.rs`

## Delivery Phases

### Phase 1

Extract the current Dino implementation out of `src/ui.rs` into `src/dino/` without changing behavior.

### Phase 2

Move to a logical 600x150 simulation space with a fixed 60 Hz update loop.

### Phase 3

Replace the current simplified physics with Chromium-style jump, jump release, duck, and speed drop behavior.

### Phase 4

Replace obstacle spawning with rule-driven generation:

- delayed first obstacle
- dynamic gaps
- repetition limits
- pterodactyl threshold

### Phase 5

Upgrade collision to multi-hitbox narrow-phase.

### Phase 6

Extend `src/display.rs` with normalized Dino poses per species:

- waiting
- running
- jumping
- ducking
- crashed

### Phase 7

Add score milestones, polish layers, and tuning in a live terminal.

## Acceptance Criteria

- Dino code no longer lives directly in `src/ui.rs`
- the game still works through the existing `Games` tab
- save persistence remains in `src/save.rs`
- the main monster remains the runner and XP recipient
- future physics/render work can happen inside `src/dino/` with minimal `ui.rs` churn
