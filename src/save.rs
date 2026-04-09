use crate::monster::Monster;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub monster: Monster,
}

const SAVE_VERSION: u32 = 1;

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

pub fn load() -> io::Result<Option<Monster>> {
    let path = save_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)?;
    let save: SaveFile = serde_json::from_str(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(save.monster))
}

pub fn save(monster: &Monster) -> io::Result<()> {
    let path = save_path()?;
    let save = SaveFile {
        version: SAVE_VERSION,
        monster: monster.clone(),
    };
    let data = serde_json::to_string_pretty(&save)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)?;
    Ok(())
}
