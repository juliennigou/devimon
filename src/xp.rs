use crate::monster::Monster;
use crate::save::devimon_dir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

/// Cap on XP awarded per minute of wall-clock time.
const XP_PER_MINUTE_CAP: u32 = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct XpEvent {
    pub kind: String,
    pub path: String,
    pub timestamp: DateTime<Utc>,
}

pub fn events_path() -> io::Result<PathBuf> {
    Ok(devimon_dir()?.join("events.json"))
}

pub fn append_event(event: &XpEvent) -> io::Result<()> {
    let path = events_path()?;
    let mut events: Vec<XpEvent> = if path.exists() {
        let data = fs::read_to_string(&path)?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };
    events.push(XpEvent {
        kind: event.kind.clone(),
        path: event.path.clone(),
        timestamp: event.timestamp,
    });
    let data = serde_json::to_string(&events)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)?;
    Ok(())
}

pub fn load_and_clear_events() -> io::Result<Vec<XpEvent>> {
    let path = events_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    let events: Vec<XpEvent> = serde_json::from_str(&data).unwrap_or_default();
    fs::write(&path, "[]")?;
    Ok(events)
}

/// Drain the buffered file-modification events and translate them to XP.
/// Returns the total XP awarded so the caller can show it to the user.
pub fn drain_and_apply(monster: &mut Monster) -> io::Result<u32> {
    let events = load_and_clear_events()?;
    if events.is_empty() {
        return Ok(0);
    }

    // Group events by minute bucket, then cap each bucket.
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<i64, u32> = BTreeMap::new();
    for ev in &events {
        let minute = ev.timestamp.timestamp() / 60;
        *buckets.entry(minute).or_insert(0) += 1;
    }

    let mut total_xp: u32 = 0;
    for (_minute, count) in buckets {
        // Base: one XP per file modification.
        let mut xp = count;
        // Burst bonus: 3+ files in the same minute adds +2.
        if count >= 3 {
            xp += 2;
        }
        // All-needs-high multiplier.
        if monster.hunger > 70.0 && monster.energy > 70.0 && monster.mood > 70.0 {
            xp = (xp as f32 * 1.25).round() as u32;
        }
        // Too-tired brake.
        if monster.energy < 10.0 {
            xp = 0;
        }
        // Per-minute cap.
        if xp > XP_PER_MINUTE_CAP {
            xp = XP_PER_MINUTE_CAP;
        }
        total_xp += xp;
    }

    if total_xp > 0 {
        monster.gain_xp(total_xp);
        // Working with your pet lifts its mood a little.
        let bump = (total_xp as f32 / 5.0).min(5.0);
        let new_mood = monster.mood + bump;
        monster.set_mood(new_mood);
        monster.last_active = Utc::now();
    }

    Ok(total_xp)
}

/// Apply passive decay first, then drain the queued file events into XP.
pub fn tick_monster_progress(monster: &mut Monster) -> io::Result<(bool, u32)> {
    let decayed = monster.apply_decay();
    let xp_gained = drain_and_apply(monster)?;
    Ok((decayed, xp_gained))
}
