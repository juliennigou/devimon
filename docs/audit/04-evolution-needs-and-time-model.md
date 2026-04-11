# Evolution, Needs, And Time Model

## Problem

The pet-care layer is currently readable, but several mechanics are too loose or incorrect to support meaningful progression gates.

## Current Issues

### Baby To Young Gate

The implementation checks `peak_mood > 50`, but monsters spawn at `80` mood. That makes the first evolution gate effectively "reach level 5".

### Young To Evolved Gate

The intended "average mood over last 7 days" is not time-based. Samples are appended on each decay call, not on a real hourly cadence, so the buffer can be filled far faster than 7 days.

### Care Pressure

- spawn starts high on all needs
- first action cooldowns are already bypassed
- feed, play, and rest have large flat restores

This makes the care loop forgiving, which is good, but also close to trivial.

## Goal

Keep the game non-punitive while making care meaningful enough to shape evolution and progression style.

## Recommended Changes

### Evolution Gates

Baby to Young should require actual care history, for example:

- level threshold
- at least one feed, one play, and one rest
- minimum recent average mood

Young to Evolved should use true time-based metrics, for example:

- level threshold
- 7-day rolling mood average derived from timestamped samples
- minimum number of active days

### Time Sampling

Replace `Vec<f32>` mood samples with timestamped samples or aggregated hourly buckets.

Examples:

- hourly mood aggregates
- daily summaries
- rolling 7-day derived metrics

### Needs Balance

Keep the philosophy of "no brutal punishment", but make neglect visibly matter:

- mild XP penalties when key needs are low
- stronger loss of multiplier access
- maybe reduced mood recovery when hunger or energy are poor

## Ordering Fix

Decay must be applied before XP and evolution checks.

That single ordering change improves fairness immediately.

## Acceptance Criteria

- First evolution is not automatic from default spawn state.
- The final evolution gate really reflects multi-day care quality.
- Care actions matter without turning the pet into a chore.
- The code matches the documented design.
