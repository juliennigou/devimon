# Devimon Audit Tracks

This folder splits the audit into concrete fix tracks so each issue can move from analysis to implementation without losing scope.

## Documents

- `01-server-authoritative-leaderboard.md`
  Make ranking and progression defensible by moving trust away from the client.
- `02-sync-identity-and-monster-ownership.md`
  Fix account, device, and monster ownership rules so sync cannot overwrite or hijack other monsters.
- `03-xp-progression-integrity.md`
  Rework XP intake so the game rewards real coding activity instead of raw event noise.
- `04-evolution-needs-and-time-model.md`
  Fix evolution gates, decay ordering, and mood sampling so progression matches the intended pet loop.
- `05-dino-monster-unlock-rules.md`
  Redefine Dino as a monster-unlock signal instead of a direct XP farm.
- `06-testing-and-observability.md`
  Add the tests and telemetry needed to trust progression and anti-cheat logic.

## Priority

1. Server-authoritative leaderboard
2. Sync identity and monster ownership
3. XP progression integrity
4. Evolution, needs, and time model
5. Testing and observability
6. Dino monster unlock rules

## Product Direction

The core rule stays the same:

> The main monster should grow from real coding activity, whether that work is done manually or accelerated with AI.

Competitive features must therefore be built around verifiable coding-derived signals, not fully client-declared state.
