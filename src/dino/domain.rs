#[derive(Clone, Copy)]
pub struct DinoObstacle {
    pub x: f32,
    pub altitude: f32,
    pub kind: DinoObstacleKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DinoObstacleKind {
    SmallCactus,
    LargeCactus,
    Pterodactyl,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DinoGamePhase {
    Ready,
    Starting,
    Running,
    Crashed,
    Paused,
    Exiting,
}

pub struct DinoGameSession {
    pub elapsed_ms: u64,
    pub frame: u64,
    pub runner_altitude: f32,
    pub runner_velocity: f32,
    pub obstacles: Vec<DinoObstacle>,
    pub next_spawn_in_steps: u16,
    pub rng_state: u64,
    pub phase: DinoGamePhase,
    pub jump_held: bool,
    pub duck_held: bool,
    pub duck_hold_steps: u8,
    pub speed_drop: bool,
    pub current_speed: f32,
    pub distance_ran: f32,
    pub score: u32,
    pub phase_steps: u16,
    pub crash_elapsed_ms: u64,
    pub pending_jump: bool,
    pub last_spawn_kind: Option<DinoObstacleKind>,
    pub repeated_spawn_count: u8,
}

impl DinoObstacleKind {
    pub fn width(self) -> usize {
        match self {
            DinoObstacleKind::SmallCactus => 4,
            DinoObstacleKind::LargeCactus => 5,
            DinoObstacleKind::Pterodactyl => 6,
        }
    }

    pub fn sprite(self, frame: u64) -> &'static [&'static str] {
        match self {
            DinoObstacleKind::SmallCactus => &["  | ", " _|_", "| | ", "|_| "],
            DinoObstacleKind::LargeCactus => &["  |  ", " _|_ ", "| | |", "|_|_|"],
            DinoObstacleKind::Pterodactyl => {
                if (frame / 5).is_multiple_of(2) {
                    &[" /^^\\ ", "<_()_>", " /||\\ ", "  vv  "]
                } else {
                    &[" /^\\  ", "<_()_>", " /||\\\\", "  vv  "]
                }
            }
        }
    }

    pub fn logical_width(self) -> f32 {
        self.width() as f32 * super::UNITS_PER_CELL
    }

    pub fn is_cactus(self) -> bool {
        matches!(
            self,
            DinoObstacleKind::SmallCactus | DinoObstacleKind::LargeCactus
        )
    }
}
