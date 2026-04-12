# Verified / Unverified Leaderboard

## Problem

The current anti-cheat redesign is safer, but it creates a product problem:

- local monster progression remains the real pet progression
- cloud ranked progression is server-capped and trusted
- those two levels can diverge sharply

This is technically defensible, but confusing for players.

Example:

- a player levels a monster offline to level 20
- they log in later
- local monster still shows level 20
- cloud leaderboard may show level 1 or 3 because only recent trusted evidence counts

That feels like a reset even when the local pet is real.

## Goal

Keep a level-based leaderboard, but make trust explicit without punishing offline players.

The leaderboard should remain fun and readable while clearly showing whether a monster's cloud progression is trusted.

## Product Decision

Use only two trust states:

- `Verified`
- `Unverified`

Do not introduce a third state such as `Legacy`.

## Meaning Of Each State

### Verified

The system has enough trusted evidence to treat the cloud progression as competitive.

Examples:

- progression accumulated through normal authenticated cloud sync
- progression that passes the verification rules for trusted activity

### Unverified

The monster is synced to cloud, but its progression cannot yet be trusted as competitive.

Examples:

- monster imported after a long offline history
- monster with insufficient trusted activity evidence
- monster whose verification checks fail or remain incomplete

`Unverified` must not be presented as cheating. It only means the system cannot fully verify that progression yet.

## Why This Is Better

- preserves the level fantasy
- avoids hard resets for offline players
- keeps leaderboard anti-cheat signals understandable
- lets players keep the same pet identity locally and in cloud
- avoids the awkward "local level 41, cloud level 3" split as the main product story

## Recommended Model

Cloud sync should store:

- cloud monster level
- cloud total XP
- cloud stage
- trust status: `Verified` or `Unverified`

The leaderboard should display level as usual, but attach the trust badge to every entry.

Examples:

- `Nyx · Lv. 20 · Verified`
- `Bitu · Lv. 20 · Unverified`

## Ranking Policy

There are two valid product choices.

### Option A

Show everyone in one leaderboard, but allow filtering to `Verified only`.

Pros:

- simplest social view
- unverified players still feel included
- preserves visibility for offline players

Cons:

- mixed ranking can feel less competitive unless the filter is prominent

### Option B

Only `Verified` monsters receive official rank numbers.

`Unverified` monsters may still appear, but:

- they do not receive official placement
- they are visually marked as unverified

Pros:

- strongest competitive integrity
- clearest distinction for serious players

Cons:

- slightly harsher for offline-first players

## Recommendation

Implement Option B.

That gives the cleanest competitive story:

- cloud progression is visible for everyone
- official placement belongs to verified entries only
- offline or newly imported players are not deleted or hidden
- verification remains something a player can earn

## Verification Model

When a player first links cloud after offline progression, the system should:

1. sync the monster state to cloud
2. mark the monster `Unverified`
3. run a verification pass on available evidence
4. upgrade the monster to `Verified` only if the checks pass

If the verification pass fails or evidence is weak:

- keep the monster synced
- keep it `Unverified`
- continue collecting trusted evidence through future online syncing

## Important Constraint

Offline logs are not fully trustworthy by default.

A verification script can only establish:

- plausible enough to accept as verified

or

- not strong enough to verify

It cannot prove historical offline progression with certainty unless the evidence itself is tamper-resistant.

So the system should treat verification as:

- sufficient trusted evidence

not:

- mathematical proof

## Proposed Verification Inputs

The first pass can use:

- local event backlog shape
- XP history consistency
- timestamp monotonicity
- impossible XP spikes
- gaps inconsistent with claimed progression
- save file coherence with event history

Over time, stronger verification can come from:

- authenticated sync continuity
- consistent device history
- repeated successful ranked syncs
- absence of suspicious sync findings

## UI Changes

### Local App

The cloud section should show:

- cloud level
- trust badge
- a short explanation when unverified

Suggested copy:

- `Cloud status: Unverified`
- `This monster is synced, but its progression is not yet verified for official ranking.`

### Leaderboard Site

Add:

- badge next to each entry
- `Verified only` filter
- short legend explaining the two states

Suggested legend:

- `Verified: eligible for official ranking`
- `Unverified: synced, but not yet verified`

## Data Model Direction

Add explicit trust fields instead of inferring trust indirectly from ranked XP mechanics.

Suggested fields:

- `verification_status` with values `verified` or `unverified`
- `verified_at` nullable timestamp
- `verification_reason` nullable short enum or string

Possible reasons:

- `trusted_sync_history`
- `import_pending_verification`
- `verification_failed`
- `suspicious_activity`

The UI should primarily expose only the status, not the raw reason.

## Migration Direction

For current users:

1. keep synced monsters visible
2. initialize trust status conservatively
3. mark existing imported cloud state as `Unverified` unless sufficient trusted evidence exists
4. allow future online activity to move the monster into `Verified`

This avoids pretending old imported progress is fully trusted while also avoiding visible punishment.

## Relationship To Current Ranked XP Work

The current server-capped `ranked_xp_delta` pipeline still has value.

It should remain part of verification and anti-cheat.

But it should no longer be the only player-facing story for cloud progression.

Instead:

- cloud progression stays visible
- trust status explains whether it is competitive
- verified ranking uses the trusted evidence pipeline

## Acceptance Criteria

- A player can sync an offline-raised monster without feeling reset.
- Leaderboard entries clearly show `Verified` or `Unverified`.
- Official ranking excludes or filters out `Unverified` entries.
- The app explains unverified status without implying misconduct.
- Existing anti-cheat telemetry still applies to verification decisions.

## Next Implementation Step

Implement the trust-status data model and UI first.

Do not start with a complex verification script.

The first milestone should be:

- explicit `Verified` / `Unverified` cloud status
- official ranking based on `Verified` only
- unverified entries still visible

After that, add the verification pass that upgrades monsters from `Unverified` to `Verified`.
