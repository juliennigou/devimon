# Implementation Milestones

This file tracks concrete progress on the anti-cheat, progression-integrity, and transparency roadmap.

## 2026-04-11

### Milestone 1

- Added audit workstream docs under `docs/audit/`.
- Hardened worker sync ownership so client-supplied `monster_id` no longer controls cloud ownership.
- Added first-pass worker validation for impossible snapshot combinations.
- Added schema-level checks for basic monster invariants.

Commits:

- `4200ce8` `fix(worker): harden sync ownership and validation`
- `d64aadc` `feat: tighten progression integrity and dino unlocks`

### Milestone 2

- Applied decay before XP in CLI and TUI progression paths.
- Fixed the first evolution gate so it is not automatically satisfied from spawn state.
- Removed Dino direct XP rewards.
- Replaced Dino rewards with queued unlock triggers:
  first new record once
  later runs above `120s`
- Added Rust tests around progression and Dino unlock behavior.

### Milestone 3

- Introduced server-owned ranked progression fields for leaderboard trust.
- Leaderboard now orders by trusted ranked XP and trusted ranked level.
- Trusted ranked XP is capped by server-observed time since the previous sync.
- Added worker tests for ranked progression derivation and capping.

Commit:

- `89784b0` `feat(worker): add authoritative ranked progression`

### Milestone 4

- Surfaced trusted cloud progression in the CLI and TUI.
- Stored trusted cloud rank, level, stage, and accepted sync delta locally.
- Made local UX explicitly distinguish local monster progress from trusted cloud progress.

Commit:

- `eadc095` `feat: surface trusted cloud progression`

### Milestone 5

- Surfaced requested versus accepted ranked XP on sync.
- Added local visibility when the server caps ranked XP growth.
- Stored milestone progress in-repo.

Commit:

- `1c00584` `feat: track sync caps and milestone progress`

### Milestone 6

- Replaced ranked progression inference from local monster totals with explicit `ranked_xp_delta` evidence.
- Kept unaccepted ranked XP locally pending for later syncs instead of discarding it.
- Added suspicious-sync telemetry in the worker for capped, zero-elapsed, and implausible ranked XP submissions.
- Added worker tests for suspicious-sync detection.

Commits:

- `7996b96` `feat: sync ranked xp as coding evidence`

Pending next:

- expose suspicious-sync visibility to operators or admin tooling
- stop using client-declared level/stage as ranked inputs entirely
