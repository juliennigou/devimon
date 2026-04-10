pub mod collision;
pub mod domain;
pub mod input;
pub mod integration;
pub mod render;
pub mod update;

pub use collision::has_collision;
pub use domain::{DinoGamePhase, DinoGameSession};
pub use input::DinoCommand;
pub use integration::crash;
pub use render::{build_world, format_duration_ms, status_text};

pub const LOGICAL_WIDTH: f32 = 600.0;
pub const LOGICAL_HEIGHT: f32 = 150.0;
pub const GROUND_HEIGHT_ROWS: usize = 2;
pub const SIM_STEP: std::time::Duration = std::time::Duration::from_nanos(16_666_667);
pub const XP_INTERVAL_MS: u64 = 10_000;
pub const RUNNER_X: f32 = 70.0;
pub const UNITS_PER_CELL: f32 = 10.0;
