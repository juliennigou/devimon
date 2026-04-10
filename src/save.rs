use crate::monster::Monster;
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
    pub sync_dirty: bool,
}

impl Default for CloudState {
    fn default() -> Self {
        Self {
            device_id: new_device_id(),
            monster_id: None,
            account: None,
            last_synced_at: None,
            sync_dirty: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DinoGameProgress {
    #[serde(default)]
    pub best_time_ms: u64,
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

const SAVE_VERSION: u32 = 4;

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
    let base = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
    let dir = base.join(".devimon");
    fs::create_dir_all(&dir)?;
    Ok(dir)
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
    state.cloud.sync_dirty = false;
}
