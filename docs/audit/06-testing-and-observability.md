# Testing And Observability

## Problem

The progression system has almost no automated protection. The current tests only cover watcher ignore rules.

That is far too little for a game with progression, sync, and anti-cheat concerns.

## Goal

Add enough automated coverage and runtime visibility to trust progression logic during future refactors.

## Priority Test Areas

### XP Engine

- minute bucketing
- repeated edits to one file
- per-minute caps
- low-energy XP blocking
- ordering with decay

### Evolution

- first evolution gate
- final evolution gate
- rolling mood calculations
- impossible stage submissions

### Sync And Leaderboard

- ownership enforcement
- rejection of forged monster ids
- rejection of impossible XP jumps
- deterministic rank ordering

### Dino

- first-record unlock trigger
- 120-second trigger rule
- no spawn for now
- no main-monster XP leak

## Runtime Observability

Add structured logging for:

- accepted syncs
- rejected syncs
- suspicious activity batches
- authoritative XP grants
- unlock trigger creation

## Suggested Tooling

- Rust unit tests for local progression logic
- Worker tests for API invariants
- fixture-based integration tests for sync flows

## Acceptance Criteria

- Each progression rule has at least one direct test.
- Each anti-cheat invariant has at least one rejection test.
- Sync failures are diagnosable from logs.
- Future changes to progression cannot silently alter fairness.
