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
- `bac0fed` `feat(worker): log suspicious ranked syncs`

### Milestone 7

- Added a lightweight admin/debug endpoint for recent suspicious syncs.
- Protected the endpoint with `ADMIN_DEBUG_TOKEN`.
- Added query filters for `limit`, `account_id`, and `severity`.
- Added worker tests around the admin query and auth helpers.

### Milestone 8

- Fully decoupled ranked truth from client monster snapshot consistency.
- Worker snapshot validation now treats client level/xp/total_xp/stage as profile data only.
- Ranked progression remains derived exclusively from trusted `ranked_xp_delta` evidence.
- Added worker tests proving profile snapshot inconsistency no longer affects ranked trust.

Commit:

- `5f3f196` `refactor(worker): decouple ranked truth from profile snapshot`

### Milestone 9

- Documented the admin/debug suspicious-sync workflow for operators.
- Added the required `ADMIN_DEBUG_TOKEN` secret to the Cloudflare README.
- Added a dedicated operator note with request examples, filters, and reason meanings.

Commit:

- `1c697e6` `docs: add admin debug operator guide`

### Milestone 10

- Trimmed the sync snapshot down to true profile fields only.
- Worker now mirrors ranked fields from trusted ranked progression instead of client monster progression.
- Made mood-history sampling time-based with one sample per elapsed hour, instead of one sample per tick.
- Added tests for profile-only snapshot validation and hourly mood sampling.

Pending next:

- revisit final evolution mood averaging with stronger multi-day semantics

### Milestone 11

- Fixed the D1 deploy/bootstrap path for existing remote databases.
- Removed ranked index creation from `schema.sql`, which was unsafe against older `monsters` tables.
- Moved ranked leaderboard index creation into the Worker's lazy runtime migration helper.
- Updated Cloudflare deployment docs to describe `schema.sql` as bootstrap schema plus runtime backfills.

### Milestone 12

- Defined the next leaderboard product model around explicit `Verified` and `Unverified` trust status.
- Kept the leaderboard level-based instead of switching to a score-only competitive view.
- Recommended official ranking for `Verified` entries only, while keeping `Unverified` monsters visible.
- Recorded the trust-model direction as a follow-up audit track and checklist item.

### Milestone 13

- Implemented explicit `Verified` / `Unverified` trust state in the worker data model.
- Restored cloud display progression for synced monsters while keeping official ranking verified-only.
- Added official-rank filtering rules so unverified monsters stay visible without receiving rank placement.
- Updated the Rust client and TUI to show cloud progression, verification status, and official rank separately.
- Updated the leaderboard site with trust badges and a `Verified only` filter.
