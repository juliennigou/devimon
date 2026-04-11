# Sync Identity And Monster Ownership

## Problem

The current sync model allows dangerous ownership ambiguity between account, device, local monster, and remote monster.

Today the server accepts a client-provided `monster_id`, and the account-to-monster mapping is too permissive. That creates a path for monster hijacking and accidental overwrites.

## Main Risks

- A user can target another user's public `monster_id`.
- One sync can reassign ownership during upsert.
- Local "main monster" and "leaderboard monster" are not the same concept.
- The cloud currently assumes one monster per account, while local save supports a collection.

## Product Decision Needed

The project must choose one of these models:

1. One ranked monster per account
2. Many monsters per account, one selected as ranked

Given the current local collection system, option 2 is the better long-term model.

## Recommended Direction

### Ownership Rules

- A monster id is server-issued only.
- A monster belongs to exactly one account.
- Ownership cannot be reassigned by normal sync.
- Device id proves origin scope, not ownership.
- A local monster can be linked to a remote monster only through an explicit server-validated bind flow.

### Linking Rules

- First sync creates a remote monster if no link exists.
- Subsequent syncs update only the monster already linked to that local monster.
- Switching the ranked monster should be an explicit action, not an implicit "highest local monster wins" rule.

## Changes To Make

- Remove client ability to choose a new authoritative `monster_id`.
- Add a mapping between local monster id and remote monster id.
- Separate:
  active monster for gameplay
  selected ranked monster for cloud sync
- Add server checks:
  remote monster must belong to authenticated account
  unknown monster ids are rejected unless created by server flow

## API Direction

### Replace Current Behavior

Avoid a generic sync that can create or overwrite based on arbitrary ids.

### Introduce Explicit Endpoints

- `POST /api/monsters`
  create remote monster, returns server id
- `POST /api/monsters/:id/link-device`
  optional device bind or trust flow
- `POST /api/monsters/:id/activity`
  submit activity evidence
- `POST /api/ranked-monster/select`
  choose which monster is shown on the leaderboard

## Local Save Changes

Add explicit fields for:

- `remote_monster_id` per monster
- `selected_ranked_monster_id`

Do not infer ranked selection from highest level.

## Acceptance Criteria

- A public leaderboard entry cannot be hijacked by reusing its id.
- Sync updates only monsters owned by the authenticated account.
- A user collection can exist locally without ambiguous cloud behavior.
- The ranked monster is explicit and stable.
