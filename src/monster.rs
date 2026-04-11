use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn new_monster_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Species {
    /// Fire line: Embit → Pyrofang → Infernox
    #[default]
    Ember,
    /// Water line: Driplet → Wavekin → Maelstryx
    Tide,
    /// Grass line: Sprout → Vinekith → Eldroak
    Bloom,
}

impl Species {
    /// Display label for the species line itself.
    pub fn label(self) -> &'static str {
        match self {
            Species::Ember => "Ember",
            Species::Tide => "Tide",
            Species::Bloom => "Bloom",
        }
    }

    /// Display name of the specific evolution form (species + stage).
    pub fn form_name(self, stage: Stage) -> &'static str {
        match (self, stage) {
            (Species::Ember, Stage::Baby) => "Embit",
            (Species::Ember, Stage::Young) => "Pyrofang",
            (Species::Ember, Stage::Evolved) => "Infernox",
            (Species::Tide, Stage::Baby) => "Driplet",
            (Species::Tide, Stage::Young) => "Wavekin",
            (Species::Tide, Stage::Evolved) => "Maelstryx",
            (Species::Bloom, Stage::Baby) => "Sprout",
            (Species::Bloom, Stage::Young) => "Vinekith",
            (Species::Bloom, Stage::Evolved) => "Eldroak",
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        match input.to_lowercase().as_str() {
            "ember" | "fire" => Ok(Species::Ember),
            "tide" | "water" => Ok(Species::Tide),
            "bloom" | "grass" => Ok(Species::Bloom),
            other => Err(format!(
                "unknown species '{}' — try: ember, tide, bloom",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Stage {
    Baby,
    Young,
    Evolved,
}

impl Stage {
    pub fn label(&self) -> &'static str {
        match self {
            Stage::Baby => "Baby",
            Stage::Young => "Young",
            Stage::Evolved => "Evolved",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monster {
    #[serde(default = "new_monster_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub species: Species,
    pub level: u32,
    pub xp: u32,
    #[serde(default)]
    pub total_xp: u32,
    pub stage: Stage,

    pub hunger: f32,
    pub energy: f32,
    pub mood: f32,

    pub last_fed: DateTime<Utc>,
    pub last_played: DateTime<Utc>,
    pub last_rested: DateTime<Utc>,
    pub last_decay: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub created_at: DateTime<Utc>,

    // Tracks the highest value each need has ever reached — used for evolution gates.
    pub peak_hunger: f32,
    pub peak_energy: f32,
    pub peak_mood: f32,

    // Rolling mood samples for evolved-stage gate.
    pub mood_samples: Vec<f32>,
}

impl Monster {
    pub fn spawn(name: String) -> Self {
        let now = Utc::now();
        // Offset action timestamps into the past so the cooldowns don't block
        // the player's very first interaction with a freshly spawned monster.
        let long_ago = now - chrono::Duration::days(1);
        Self {
            id: new_monster_id(),
            name,
            species: Species::Ember,
            level: 1,
            xp: 0,
            total_xp: 0,
            stage: Stage::Baby,
            hunger: 80.0,
            energy: 80.0,
            mood: 80.0,
            last_fed: long_ago,
            last_played: long_ago,
            last_rested: long_ago,
            last_decay: now,
            last_active: now,
            created_at: now,
            peak_hunger: 80.0,
            peak_energy: 80.0,
            peak_mood: 80.0,
            mood_samples: vec![80.0],
        }
    }

    pub fn spawn_with_species(name: String, species: Species) -> Self {
        let mut monster = Self::spawn(name);
        monster.species = species;
        monster
    }

    /// XP needed to reach the next level. Grows linearly for predictability.
    pub fn xp_to_next(&self) -> u32 {
        10 + self.level * 5
    }

    /// Add XP and cascade level-ups.
    pub fn gain_xp(&mut self, amount: u32) {
        self.xp += amount;
        self.total_xp += amount;
        while self.xp >= self.xp_to_next() {
            self.xp -= self.xp_to_next();
            self.level += 1;
        }
    }

    /// Clamp a need to [0, 100] and update peak tracking.
    pub fn set_hunger(&mut self, v: f32) {
        self.hunger = v.clamp(0.0, 100.0);
        if self.hunger > self.peak_hunger {
            self.peak_hunger = self.hunger;
        }
    }

    pub fn set_energy(&mut self, v: f32) {
        self.energy = v.clamp(0.0, 100.0);
        if self.energy > self.peak_energy {
            self.peak_energy = self.energy;
        }
    }

    pub fn set_mood(&mut self, v: f32) {
        self.mood = v.clamp(0.0, 100.0);
        if self.mood > self.peak_mood {
            self.peak_mood = self.mood;
        }
    }

    /// Apply passive time-based decay based on elapsed hours since last update.
    pub fn apply_decay(&mut self) {
        let now = Utc::now();
        let elapsed_hours = (now - self.last_decay).num_seconds() as f32 / 3600.0;
        if elapsed_hours <= 0.0 {
            return;
        }

        self.set_hunger(self.hunger - 5.0 * elapsed_hours);
        self.set_energy(self.energy - 3.0 * elapsed_hours);
        let mood_decay = if self.hunger < 20.0 { 4.0 } else { 2.0 };
        self.set_mood(self.mood - mood_decay * elapsed_hours);

        // Sample mood once per apply, cap the buffer at ~7 days of hourly samples.
        self.mood_samples.push(self.mood);
        if self.mood_samples.len() > 168 {
            let drop = self.mood_samples.len() - 168;
            self.mood_samples.drain(0..drop);
        }

        self.last_decay = now;
    }

    pub fn avg_mood(&self) -> f32 {
        if self.mood_samples.is_empty() {
            return self.mood;
        }
        let sum: f32 = self.mood_samples.iter().sum();
        sum / self.mood_samples.len() as f32
    }

    /// Check if the monster should evolve and update its stage.
    pub fn check_evolution(&mut self) -> Option<Stage> {
        let new_stage = match self.stage {
            Stage::Baby if self.level >= 5 && self.peak_mood > 50.0 => Some(Stage::Young),
            Stage::Young if self.level >= 15 && self.avg_mood() > 60.0 => Some(Stage::Evolved),
            _ => None,
        };
        if let Some(s) = new_stage {
            self.stage = s;
            Some(s)
        } else {
            None
        }
    }
}
