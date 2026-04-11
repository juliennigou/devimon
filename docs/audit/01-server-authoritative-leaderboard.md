# Server-Authoritative Leaderboard

## Problem

The current leaderboard trusts client-submitted monster state. A client can submit arbitrary `level`, `xp`, `total_xp`, `stage`, and timestamps, then appear legitimate on the leaderboard.

This is the single largest integrity issue in the project.

## Why It Matters

- The leaderboard is presented as competitive.
- Competitive ranking without authoritative validation invites trivial cheating.
- Once cheating becomes public, the feature loses product value quickly.

## Current Weak Points

- The Rust client sends a full monster snapshot during sync.
- The Worker mostly validates shape, not truth.
- The database stores the submitted state as canonical.
- Rank is computed directly from that stored client state.

## Abuse Cases

- Edit `~/.devimon/save.json` and sync.
- Call `/api/sync` directly with fabricated values.
- Inflate `total_xp` while keeping believable `level`.
- Submit impossible stage transitions.
- Backdate or fabricate activity timestamps.

## Goal

The server should become the source of truth for all leaderboard-relevant progression.

## Proposed Direction

### Phase 1

Keep local pet state for UX, but treat cloud ranking as separate authoritative progression.

The client may submit:

- authenticated account
- stable device id
- raw coding activity events or pre-validated summaries
- optional local display state for non-ranked profile views

The server should own:

- ranked XP
- ranked total XP
- ranked level
- ranked evolution stage
- ranked unlocks
- anti-cheat decisions

### Phase 2

Move from "client submits state" to "client submits evidence".

Examples:

- per-minute activity summaries
- unique file count
- project session windows
- signed or rate-limited device event batches

### Phase 3

Use server-side derivation for leaderboard projection:

- XP is recomputed server-side
- level is recomputed from authoritative XP
- stage is recomputed from authoritative gates
- suspicious sessions are flagged or ignored

## Minimum Changes Required

- Replace snapshot-driven rank updates with server-derived progression.
- Store raw or summarized activity batches separately from derived monster state.
- Add invariant validation:
  level must match total XP
  stage must match server-side gates
  XP cannot jump beyond allowed limits
- Add anti-replay rules for device submissions.
- Add rate limits per account and device.

## Suggested Data Model

### New Tables

- `activity_batches`
- `activity_minutes`
- `ranked_progression_snapshots`
- `suspicious_syncs`

### Existing Table Changes

`monsters` should become either:

- a server-owned ranked projection table

or

- a profile table that is no longer trusted for rank

## Ranking Policy

Rank should be derived from:

- authoritative total XP
- authoritative level
- deterministic tie-breakers

Tie-breakers should not depend on client-controlled timestamps.

## Migration Strategy

1. Freeze the current sync contract for backward compatibility.
2. Introduce a new server-side progression pipeline behind a feature flag.
3. Mark old client-submitted values as untrusted.
4. Recompute leaderboard data from authoritative inputs.
5. Remove snapshot-authoritative ranking.

## Acceptance Criteria

- Editing local save data cannot improve leaderboard rank.
- Direct API calls with fabricated XP cannot improve leaderboard rank.
- Server-owned XP, level, and stage stay internally consistent.
- Suspicious submissions are logged and reviewable.

## Notes

This track has to land before the leaderboard is treated as fair.
