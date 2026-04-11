# XP Progression Integrity

## Problem

The current XP model is easy to game because it rewards event volume more than meaningful coding activity.

The current implementation also diverges from the stated rule set in important ways.

## Current Issues

### Event Semantics

- XP is based on event count, not unique-file contribution.
- Repeated edits to one file can farm XP.
- The burst bonus can be triggered by noisy repeated edits.
- Create and remove events count the same as genuine coding edits.

### Timing Issues

- Backlogged events are processed before decay.
- Old events may receive healthy-stat bonuses that should no longer apply.
- Energy gating is checked against stale pre-decay state.

### Queue Integrity

- Event storage uses a JSON array rewritten on each append.
- Parallel watchers can race.
- Corrupt JSON falls back to empty, which can silently drop progress evidence.

## Design Goal

Reward real coding flow:

- meaningful edits
- breadth of work
- sustained sessions

Reduce reward for:

- file-touch spam
- single-file loops
- scripted noise

## Recommended Rule Changes

### Per-Minute Scoring

Score by minute bucket using structured signals, for example:

- unique files touched
- number of distinct active minutes
- optional project breadth bonus

Suggested first-pass formula:

- first unique file in minute: `+1`
- additional unique files up to small cap: `+1` each
- no extra gain from repeated writes to the same file in the same minute
- optional small streak bonus after several active minutes

### Hard Caps

- keep a per-minute cap
- add a per-hour cap
- add diminishing returns for long single-file loops

### Ordering

Always:

1. apply decay
2. evaluate effective needs
3. award XP
4. update mood/activity
5. evaluate evolution

## Storage Direction

Replace raw `events.json` queue with one of:

- SQLite
- append-only log with compaction
- server-submitted minute summaries

SQLite is the cleanest local step if the project stays local-first.

## Anti-Abuse Heuristics

- collapse repeated writes to the same file within a minute
- cap contribution from create/delete churn
- track active minutes, not only raw events
- flag impossible sustained caps over long periods

## Acceptance Criteria

- Rewriting one file every few seconds is much less rewarding than real multi-file coding.
- Old buffered events do not use stale healthy stats.
- Parallel watcher activity does not corrupt the queue.
- XP rules in docs and code match each other.
