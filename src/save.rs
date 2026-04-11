use crate::monster::{Monster, Stage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSession {
    pub account_id: String,
    pub username: String,
    pub session_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudState {
    #[serde(default = "new_device_id")]
    pub device_id: String,
    #[serde(default)]
    pub monster_id: Option<String>,
    #[serde(default)]
    pub account: Option<AccountSession>,
    #[serde(default)]
    pub last_synced_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub trusted_total_xp: Option<u32>,
    #[serde(default)]
    pub trusted_level: Option<u32>,
    #[serde(default)]
    pub trusted_stage: Option<Stage>,
    #[serde(default)]
    pub leaderboard_rank: Option<u64>,
    #[serde(default)]
    pub last_accepted_xp_delta: Option<u32>,
    #[serde(default)]
    pub last_requested_xp_delta: Option<u32>,
    #[serde(default)]
    pub last_max_accepted_xp_delta: Option<u32>,
    #[serde(default)]
    pub sync_dirty: bool,
}

impl Default for CloudState {
    fn default() -> Self {
        Self {
            device_id: new_device_id(),
            monster_id: None,
            account: None,
            last_synced_at: None,
            trusted_total_xp: None,
            trusted_level: None,
            trusted_stage: None,
            leaderboard_rank: None,
            last_accepted_xp_delta: None,
            last_requested_xp_delta: None,
            last_max_accepted_xp_delta: None,
            sync_dirty: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DinoGameProgress {
    #[serde(default)]
    pub best_time_ms: u64,
    #[serde(default)]
    pub pending_unlock_triggers: u32,
    #[serde(default)]
    pub record_unlock_claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DinoUnlockReason {
    FirstRecord,
    Endurance,
}

pub const DINO_UNLOCK_TRIGGER_THRESHOLD_MS: u64 = 120_000;

impl DinoGameProgress {
    pub fn register_run_completion(&mut self, duration_ms: u64) -> Option<DinoUnlockReason> {
        let is_record = duration_ms > self.best_time_ms;
        let qualifies_for_endurance = duration_ms > DINO_UNLOCK_TRIGGER_THRESHOLD_MS;

        if is_record {
            if !self.record_unlock_claimed {
                self.pending_unlock_triggers = self.pending_unlock_triggers.saturating_add(1);
                self.record_unlock_claimed = true;
                self.best_time_ms = duration_ms;
                return Some(DinoUnlockReason::FirstRecord);
            }

            self.best_time_ms = duration_ms;
        }

        if qualifies_for_endurance {
            self.pending_unlock_triggers = self.pending_unlock_triggers.saturating_add(1);
            return Some(DinoUnlockReason::Endurance);
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameProgress {
    #[serde(default)]
    pub dino: DinoGameProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,

    /// All monsters in the player's collection.
    #[serde(default)]
    pub monsters: Vec<Monster>,

    /// ID of the currently active (levelling-up) monster.
    #[serde(default)]
    pub active_monster_id: String,

    /// Legacy single-monster field — read on load for migration, never written.
    #[serde(default, skip_serializing)]
    pub monster: Option<Monster>,

    #[serde(default)]
    pub cloud: CloudState,

    #[serde(default)]
    pub games: GameProgress,
}

const SAVE_VERSION: u32 = 5;

fn new_device_id() -> String {
    Uuid::new_v4().to_string()
}

impl SaveFile {
    pub fn new(monster: Monster) -> Self {
        let id = monster.id.clone();
        Self {
            version: SAVE_VERSION,
            monsters: vec![monster],
            active_monster_id: id,
            monster: None,
            cloud: CloudState::default(),
            games: GameProgress::default(),
        }
    }

    // ── Collection helpers ────────────────────────────────────────────────

    pub fn active_monster_idx(&self) -> usize {
        self.monsters
            .iter()
            .position(|m| m.id == self.active_monster_id)
            .unwrap_or(0)
    }

    pub fn active_monster(&self) -> &Monster {
        &self.monsters[self.active_monster_idx()]
    }

    pub fn active_monster_mut(&mut self) -> &mut Monster {
        let idx = self.active_monster_idx();
        &mut self.monsters[idx]
    }

    /// The monster shown on the leaderboard: highest level (total_xp as tie-breaker).
    pub fn leaderboard_monster(&self) -> &Monster {
        self.monsters
            .iter()
            .max_by_key(|m| (m.level, m.total_xp))
            .expect("monsters list is never empty")
    }

    /// Promote a monster to main by its ID (no-op if ID not found).
    pub fn set_active(&mut self, id: &str) {
        if self.monsters.iter().any(|m| m.id == id) {
            self.active_monster_id = id.to_string();
        }
    }

    /// Returns `true` if no other monster in the collection shares `name`.
    pub fn is_name_available(&self, name: &str) -> bool {
        !self
            .monsters
            .iter()
            .any(|m| m.name.eq_ignore_ascii_case(name))
    }
}

// ── Persistence ───────────────────────────────────────────────────────────────

pub fn devimon_dir() -> io::Result<PathBuf> {
    let dir = platform_devimon_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(windows)]
fn platform_devimon_dir() -> io::Result<PathBuf> {
    if let Some(base) = dirs::data_local_dir() {
        return Ok(base.join("Devimon"));
    }

    let base = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
    Ok(base.join(".devimon"))
}

#[cfg(not(windows))]
fn platform_devimon_dir() -> io::Result<PathBuf> {
    let base = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
    Ok(base.join(".devimon"))
}

pub fn save_path() -> io::Result<PathBuf> {
    Ok(devimon_dir()?.join("save.json"))
}

fn normalize(mut state: SaveFile) -> SaveFile {
    state.version = SAVE_VERSION;

    // ── Migrate old single-monster format ────────────────────────────────
    if let Some(mut old) = state.monster.take() {
        if state.monsters.is_empty() {
            if old.id.is_empty() {
                old.id = Uuid::new_v4().to_string();
            }
            state.monsters.push(old);
        }
    }

    // ── Ensure every monster has an ID ───────────────────────────────────
    for m in &mut state.monsters {
        if m.id.is_empty() {
            m.id = Uuid::new_v4().to_string();
        }
    }

    // ── Ensure active_monster_id is valid ────────────────────────────────
    let valid = !state.active_monster_id.is_empty()
        && state
            .monsters
            .iter()
            .any(|m| m.id == state.active_monster_id);
    if !valid {
        state.active_monster_id = state
            .monsters
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_default();
    }

    // ── Device ID ─────────────────────────────────────────────────────────
    if state.cloud.device_id.trim().is_empty() {
        state.cloud.device_id = new_device_id();
    }

    if state.games.dino.best_time_ms > 0 {
        state.games.dino.record_unlock_claimed = true;
    }

    state
}

pub fn load_state() -> io::Result<Option<SaveFile>> {
    let path = save_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)?;
    let save: SaveFile =
        serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(normalize(save)))
}

pub fn save_state(state: &SaveFile) -> io::Result<()> {
    let path = save_path()?;
    let data = serde_json::to_string_pretty(&normalize(state.clone()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn mark_dirty(state: &mut SaveFile) {
    state.cloud.sync_dirty = true;
}

pub fn clear_session(state: &mut SaveFile) {
    state.cloud.account = None;
    state.cloud.monster_id = None;
    state.cloud.last_synced_at = None;
    state.cloud.trusted_total_xp = None;
    state.cloud.trusted_level = None;
    state.cloud.trusted_stage = None;
    state.cloud.leaderboard_rank = None;
    state.cloud.last_accepted_xp_delta = None;
    state.cloud.last_requested_xp_delta = None;
    state.cloud.last_max_accepted_xp_delta = None;
    state.cloud.sync_dirty = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_record_claims_one_pending_unlock() {
        let mut progress = DinoGameProgress::default();

        let reason = progress.register_run_completion(95_000);

        assert_eq!(reason, Some(DinoUnlockReason::FirstRecord));
        assert_eq!(progress.best_time_ms, 95_000);
        assert_eq!(progress.pending_unlock_triggers, 1);
        assert!(progress.record_unlock_claimed);
    }

    #[test]
    fn later_long_runs_queue_endurance_unlocks() {
        let mut progress = DinoGameProgress::default();

        assert_eq!(
            progress.register_run_completion(95_000),
            Some(DinoUnlockReason::FirstRecord)
        );
        assert_eq!(progress.register_run_completion(100_000), None);
        assert_eq!(
            progress.register_run_completion(130_000),
            Some(DinoUnlockReason::Endurance)
        );
        assert_eq!(progress.best_time_ms, 130_000);
        assert_eq!(progress.pending_unlock_triggers, 2);
    }

    #[test]
    fn normalize_claims_existing_records() {
        let monster = Monster::spawn("Embit".to_string());
        let state = SaveFile {
            version: 0,
            monsters: vec![monster.clone()],
            active_monster_id: monster.id.clone(),
            monster: None,
            cloud: CloudState::default(),
            games: GameProgress {
                dino: DinoGameProgress {
                    best_time_ms: 42_000,
                    pending_unlock_triggers: 0,
                    record_unlock_claimed: false,
                },
            },
        };

        let normalized = normalize(state);
        assert!(normalized.games.dino.record_unlock_claimed);
    }
}
