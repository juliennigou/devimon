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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub monster: Monster,
    #[serde(default)]
    pub cloud: CloudState,
}

const SAVE_VERSION: u32 = 2;

fn new_device_id() -> String {
    Uuid::new_v4().to_string()
}

impl SaveFile {
    pub fn new(monster: Monster) -> Self {
        Self {
            version: SAVE_VERSION,
            monster,
            cloud: CloudState::default(),
        }
    }
}

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
