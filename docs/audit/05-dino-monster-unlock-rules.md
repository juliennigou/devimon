# Dino Monster Unlock Rules

## Product Direction

Dino should stop being a direct progression farm for the main coding monster.

The mini-game can stay, but its reward should be tied to future monster unlocks, not direct XP inflation.

## New Rule

For now, Dino should only build unlock logic and state tracking.

No actual monster spawn should happen yet.

### Unlock Logic

- First time the player sets a new record in Dino:
  mark the account or save as having earned the first Dino unlock trigger
- After that, only runs longer than `120` seconds count as Dino unlock triggers
- These triggers should be recorded, but not converted into spawned monsters until the monster system for unlock rewards is ready

## Why This Is Better

- It keeps Dino meaningful
- It removes Dino as an XP bypass
- It preserves the core fantasy that the main monster grows from coding
- It creates a future path for side monsters or special eggs

## Current Problems To Remove

- Dino currently awards XP over time
- Dino progression is unrelated to coding
- While Dino is active, normal tick-based pet updates are skipped

## Implementation Direction

### Replace XP Awarding

Remove Dino-to-main-monster XP awards.

### Add Unlock State

Track local fields such as:

- `best_time_ms`
- `first_record_unlock_pending`
- `long_run_unlock_count`
- `last_unlock_run_ms`

The exact names can change, but the state should clearly separate:

- score history
- unlock triggers
- actual spawned rewards

### Deferred Reward Layer

Add a placeholder pathway:

- record unlock trigger exists
- long-run unlock trigger exists
- monster reward generation is not yet enabled

## Server Considerations

If Dino unlocks ever become competitive or cloud-visible, they must also be server-validated. For now they can remain local if they have no rank impact.

## Acceptance Criteria

- Dino no longer increases main monster XP
- first new record creates an unlock trigger only
- later runs only trigger when duration exceeds 120 seconds
- no monster is actually spawned yet
- the code structure is ready for future reward wiring
