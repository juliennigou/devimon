use crate::monster::Monster;
use chrono::{Duration, Utc};

/// Feed the monster. Returns a success message or an error string explaining
/// why the action was blocked.
pub fn feed(monster: &mut Monster) -> Result<String, String> {
    let now = Utc::now();
    let cooldown = Duration::hours(2);
    if now - monster.last_fed < cooldown {
        let remaining = cooldown - (now - monster.last_fed);
        return Err(format!(
            "{} n'a pas faim (réessaie dans {}min)",
            monster.name,
            remaining.num_minutes().max(1)
        ));
    }
    monster.set_hunger(monster.hunger + 40.0);
    monster.set_mood(monster.mood + 5.0);
    monster.last_fed = now;
    Ok(format!("🍗 {} a mangé.", monster.name))
}

pub fn play(monster: &mut Monster) -> Result<String, String> {
    let now = Utc::now();
    let cooldown = Duration::hours(1);
    if now - monster.last_played < cooldown {
        let remaining = cooldown - (now - monster.last_played);
        return Err(format!(
            "{} est déjà bien stimulé (réessaie dans {}min)",
            monster.name,
            remaining.num_minutes().max(1)
        ));
    }
    if monster.energy < 15.0 {
        return Err(format!("{} est trop fatigué pour jouer.", monster.name));
    }
    monster.set_mood(monster.mood + 30.0);
    monster.set_energy(monster.energy - 10.0);
    monster.last_played = now;
    Ok(format!("🎾 {} a joué avec toi.", monster.name))
}

pub fn rest(monster: &mut Monster) -> Result<String, String> {
    let now = Utc::now();
    let cooldown = Duration::hours(4);
    if now - monster.last_rested < cooldown {
        let remaining = cooldown - (now - monster.last_rested);
        return Err(format!(
            "{} n'a pas sommeil (réessaie dans {}min)",
            monster.name,
            remaining.num_minutes().max(1)
        ));
    }
    monster.set_energy(monster.energy + 50.0);
    monster.set_mood(monster.mood + 5.0);
    monster.last_rested = now;
    Ok(format!("💤 {} s'est reposé.", monster.name))
}
