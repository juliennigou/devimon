# Devimon Online Leaderboard Plan

## Goal

Build an online identity and leaderboard system on top of the existing terminal app without breaking the current local gameplay loop.

The implementation will happen in two phases:

1. Online identity and snapshot sync
2. Hardening for multi-device correctness and future anti-cheat

## Product Decisions For V1

- One account owns one monster.
- The leaderboard trusts client-calculated XP for the first version.
- A user can sync the same monster from multiple devices.
- `device_id` identifies one local installation, not the user.
- Local gameplay continues to work fully offline.
- Cloud sync and leaderboard participation require login.

## Identity Model

Use three IDs with different roles:

- `account_id`: the human user
- `monster_id`: the ranked pet
- `device_id`: one installed TUI instance

### Ownership Rules

- One account owns exactly one monster in v1.
- One monster belongs to exactly one account.
- One account can sync from multiple devices.
- One device is linked to one account at a time.

### ID Creation

- `device_id`: generated locally on first run and persisted in local config/save
- `account_id`: created by the backend during signup/login
- `monster_id`: assigned by the backend when the local monster is first linked to an account

## Architecture

The system will be split into four parts:

- Terminal app: local gameplay, login, snapshot sync
- API: auth, monster linking, sync, leaderboard read endpoints
- Database: account, monster, device, and sync history storage
- Website: public leaderboard UI

Flow:

`devimon client -> sync API -> database -> leaderboard website`

## Recommended Stack

- Terminal app: existing Rust codebase
- Auth: GitHub OAuth device flow
- API: Cloudflare Workers
- Database: Cloudflare D1
- Website: Cloudflare Pages

Why this stack:

- Good fit for a terminal app
- Low infrastructure cost
- Fastest path to a hosted public leaderboard
- GitHub login matches the likely audience of developers

## Phase 1: Online Identity And Snapshot Sync

### 1. Extend Local Saved State

Keep `Monster` focused on gameplay. Extend the persisted save/config layer with cloud metadata.

Planned fields:

- `device_id`
- `monster_id: Option<String>`
- `account_id: Option<String>`
- `session_token: Option<String>` or equivalent session reference
- `last_synced_at`
- `sync_dirty`

Primary file to update:

- `src/save.rs`

### 2. Generate And Persist `device_id`

On first run:

- Generate a UUID
- Save it locally
- Reuse it for all future syncs from that install

This ID is operational metadata only. It does not control ownership.

### 3. Add Account Login To The Terminal App

Add new CLI commands:

- `devimon login`
- `devimon logout`
- `devimon sync`
- `devimon whoami`

Auth flow:

1. User runs `devimon login`
2. App starts GitHub device flow
3. User opens a browser and authorizes
4. App polls until authorization completes
5. Backend returns an app session
6. Session is stored locally

Primary file to update:

- `src/main.rs`

New module:

- `src/cloud.rs` or `src/sync.rs`

### 4. Link The Local Monster To The Account

After login:

- If no remote monster exists, create one from the local monster
- Store the returned `monster_id`
- If a remote monster already exists, the remote monster becomes canonical for that account in v1

This avoids using `device_id` as the identity owner.

### 5. Add Snapshot Sync

For v1, upload snapshots instead of raw watched file events.

Reason:

- Faster to ship
- Simpler API
- Lower privacy risk
- Easier to layer onto the current app

Payload should include:

- `monster_id`
- `device_id`
- `name`
- `level`
- `xp`
- `total_xp`
- `stage`
- `hunger`
- `energy`
- `mood`
- `last_active_at`

Privacy rule:

- Do not upload raw local file paths from the watcher

### 6. Mark State As Dirty Whenever Gameplay Changes

Mark the monster state as needing sync when:

- XP is applied from local activity
- feed succeeds
- play succeeds
- rest succeeds

Then sync:

- after state-changing CLI commands
- periodically during the TUI loop
- on clean exit when possible

Files involved:

- `src/xp.rs`
- `src/actions.rs`
- `src/ui.rs`
- `src/main.rs`

### 7. Build The Backend

Cloudflare Worker endpoints:

- `POST /api/auth/github/device/start`
- `POST /api/auth/github/device/poll`
- `POST /api/monster/link`
- `POST /api/sync`
- `GET /api/leaderboard`
- `GET /api/me`

Responsibilities:

- authenticate users
- link account and monster
- accept sync payloads
- persist latest monster state
- serve leaderboard data

### 8. Build The Website

Pages to build:

- `/leaderboard`
- optional `/monster/:id`

Data to show:

- rank
- monster name
- level
- stage
- total XP
- last active

## Phase 2: Hardening

### 1. Add Sync Versioning

Each sync should carry a revision or comparable freshness marker so the backend can detect stale writes.

### 2. Add Sync History

Store accepted snapshots for:

- debugging
- auditability
- future merge analysis

### 3. Add Multi-Device Conflict Handling

V1 can use last-write-wins.

Later we can add:

- merge rules
- revision checks
- better reconciliation for concurrent devices

### 4. Decide Whether To Move XP Authority To The Server

Not part of the first implementation.

Only do this if:

- cheating becomes a real problem
- leaderboard fairness matters enough to justify the complexity

## Backend Data Model

### `accounts`

- `account_id`
- `github_user_id`
- `username`
- `created_at`

### `monsters`

- `monster_id`
- `account_id`
- `name`
- `level`
- `xp`
- `total_xp`
- `stage`
- `hunger`
- `energy`
- `mood`
- `last_active_at`
- `updated_at`

### `devices`

- `device_id`
- `account_id`
- `last_seen_at`
- `created_at`

### `sync_history`

- `id`
- `monster_id`
- `device_id`
- `received_at`
- `payload_json`

## Changes Planned In This Repo

### Existing Files

- `src/main.rs`
  - add login/logout/sync/whoami commands
  - wire sync into CLI flows

- `src/save.rs`
  - extend persisted schema for cloud metadata

- `src/monster.rs`
  - keep gameplay-focused
  - possibly add `total_xp` if needed for ranking clarity

- `src/xp.rs`
  - mark local state dirty when XP changes

- `src/actions.rs`
  - mark local state dirty on successful mutations

- `src/ui.rs`
  - add periodic sync attempts and status feedback later

### New File

- `src/cloud.rs` or `src/sync.rs`
  - auth client
  - sync client
  - request/response types

## Execution Order

1. Extend local save schema
2. Generate and persist `device_id`
3. Add `total_xp` tracking if needed
4. Add login/logout/whoami/sync CLI commands
5. Add Rust sync/auth module
6. Build Cloudflare Worker API
7. Create D1 schema
8. Build leaderboard website
9. Add TUI periodic sync polish

## Definition Of Done

- Local gameplay still works exactly as it does today
- User can log in from the terminal app
- Local monster can be linked to an online account
- `devimon sync` uploads the current monster state
- Website displays the synced monster in the leaderboard
- Repeated syncs update the same monster instead of creating duplicates

