use crate::display;

use super::{
    LOGICAL_WIDTH, UNITS_PER_CELL,
    domain::{DinoGamePhase, DinoGameSession, DinoObstacle, DinoObstacleKind},
    input::DinoCommand,
};

const FRAME_MS: u64 = 1000 / 60;
const STARTING_STEPS: u16 = 6;
const RESTART_DELAY_MS: u64 = 1200;
const START_SPEED: f32 = 6.0;
const MAX_SPEED: f32 = 13.0;
const ACCELERATION: f32 = 0.001;
const INITIAL_JUMP_VELOCITY: f32 = 10.0;
const GRAVITY: f32 = 0.6;
const MIN_JUMP_HEIGHT: f32 = 30.0;
const SPEED_DROP_COEFFICIENT: f32 = 3.0;
const INITIAL_SPAWN_DELAY_STEPS: u16 = 180;
const MIN_OBSTACLE_GAP: f32 = 90.0;
const MAX_OBSTACLE_GAP_COEFFICIENT: f32 = 1.5;
const RUN_ANIMATION_STEPS: u64 = 5;
const DUCK_ANIMATION_STEPS: u64 = 7;
const DUCK_INPUT_GRACE_STEPS: u8 = 45;

impl DinoGameSession {
    pub fn new(seed: u64) -> Self {
        Self {
            elapsed_ms: 0,
            xp_awarded: 0,
            frame: 0,
            runner_altitude: 0.0,
            runner_velocity: 0.0,
            obstacles: Vec::new(),
            next_spawn_in_steps: INITIAL_SPAWN_DELAY_STEPS,
            rng_state: seed,
            phase: DinoGamePhase::Ready,
            jump_held: false,
            duck_held: false,
            duck_hold_steps: 0,
            speed_drop: false,
            current_speed: START_SPEED,
            distance_ran: 0.0,
            score: 0,
            phase_steps: 0,
            crash_elapsed_ms: 0,
            pending_jump: false,
            last_spawn_kind: None,
            repeated_spawn_count: 0,
        }
    }

    pub fn handle_command(&mut self, command: DinoCommand) {
        match command {
            DinoCommand::JumpPressed => {
                self.jump_held = true;
                match self.phase {
                    DinoGamePhase::Ready => self.begin_run(true),
                    DinoGamePhase::Starting => self.pending_jump = true,
                    DinoGamePhase::Running => {
                        if self.is_grounded() {
                            self.launch_jump();
                        }
                    }
                    DinoGamePhase::Crashed if self.can_restart() => self.restart(true),
                    DinoGamePhase::Paused => self.phase = DinoGamePhase::Running,
                    DinoGamePhase::Crashed | DinoGamePhase::Exiting => {}
                }
            }
            DinoCommand::JumpReleased => {
                self.jump_held = false;
            }
            DinoCommand::DuckPressed => {
                self.duck_held = true;
                self.duck_hold_steps = DUCK_INPUT_GRACE_STEPS;
                match self.phase {
                    DinoGamePhase::Running if !self.is_grounded() => {
                        self.speed_drop = true;
                    }
                    DinoGamePhase::Ready => self.begin_run(false),
                    _ => {}
                }
            }
            DinoCommand::DuckReleased => {
                self.duck_held = false;
                self.duck_hold_steps = 0;
                self.speed_drop = false;
            }
            DinoCommand::Restart => match self.phase {
                DinoGamePhase::Ready => self.begin_run(false),
                DinoGamePhase::Crashed if self.can_restart() => self.restart(false),
                DinoGamePhase::Paused => self.phase = DinoGamePhase::Running,
                _ => {}
            },
            DinoCommand::TogglePause => match self.phase {
                DinoGamePhase::Running => self.phase = DinoGamePhase::Paused,
                DinoGamePhase::Paused => self.phase = DinoGamePhase::Running,
                _ => {}
            },
        }
    }

    pub fn update(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.decay_duck_hold();

        match self.phase {
            DinoGamePhase::Ready => {}
            DinoGamePhase::Starting => {
                self.phase_steps = self.phase_steps.saturating_add(1);
                if self.phase_steps >= STARTING_STEPS {
                    self.phase = DinoGamePhase::Running;
                    self.phase_steps = 0;
                    if self.pending_jump {
                        self.launch_jump();
                    }
                }
            }
            DinoGamePhase::Running => {
                self.elapsed_ms = self.elapsed_ms.saturating_add(FRAME_MS);
                self.distance_ran += self.current_speed;
                self.score = (self.distance_ran * 0.025).floor() as u32;
                self.current_speed = (self.current_speed + ACCELERATION).min(MAX_SPEED);

                self.update_runner();
                self.update_obstacles();
            }
            DinoGamePhase::Crashed => {
                self.crash_elapsed_ms = self.crash_elapsed_ms.saturating_add(FRAME_MS);
            }
            DinoGamePhase::Paused | DinoGamePhase::Exiting => {}
        }
    }

    pub fn current_pose(&self) -> display::GameSpritePose {
        if self.phase == DinoGamePhase::Crashed {
            display::GameSpritePose::Crashed
        } else if self.phase == DinoGamePhase::Ready
            || self.phase == DinoGamePhase::Starting
            || self.phase == DinoGamePhase::Paused
        {
            if self.is_ducking() {
                display::GameSpritePose::DuckA
            } else {
                display::GameSpritePose::Waiting
            }
        } else if self.is_ducking() {
            if (self.frame / DUCK_ANIMATION_STEPS).is_multiple_of(2) {
                display::GameSpritePose::DuckA
            } else {
                display::GameSpritePose::DuckB
            }
        } else if self.runner_altitude > 4.0 {
            if self.runner_velocity >= 0.0 {
                display::GameSpritePose::Jump
            } else {
                display::GameSpritePose::Fall
            }
        } else if (self.frame / RUN_ANIMATION_STEPS).is_multiple_of(2) {
            display::GameSpritePose::RunA
        } else {
            display::GameSpritePose::RunB
        }
    }

    pub fn can_restart(&self) -> bool {
        self.phase == DinoGamePhase::Crashed && self.crash_elapsed_ms >= RESTART_DELAY_MS
    }

    pub fn restart_remaining_ms(&self) -> u64 {
        RESTART_DELAY_MS.saturating_sub(self.crash_elapsed_ms)
    }

    pub fn is_ducking(&self) -> bool {
        self.duck_held && self.is_grounded()
    }

    fn next_rand(&mut self) -> u32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.rng_state >> 32) as u32
    }

    fn begin_run(&mut self, jump_on_start: bool) {
        self.elapsed_ms = 0;
        self.xp_awarded = 0;
        self.frame = 0;
        self.runner_altitude = 0.0;
        self.runner_velocity = 0.0;
        self.obstacles.clear();
        self.next_spawn_in_steps = INITIAL_SPAWN_DELAY_STEPS;
        self.phase = DinoGamePhase::Starting;
        self.speed_drop = false;
        self.duck_hold_steps = 0;
        self.current_speed = START_SPEED;
        self.distance_ran = 0.0;
        self.score = 0;
        self.phase_steps = 0;
        self.crash_elapsed_ms = 0;
        self.pending_jump = jump_on_start;
        self.last_spawn_kind = None;
        self.repeated_spawn_count = 0;
    }

    fn restart(&mut self, jump_on_start: bool) {
        self.rng_state = self
            .rng_state
            .wrapping_add(self.elapsed_ms + self.frame + 1);
        self.begin_run(jump_on_start);
    }

    fn is_grounded(&self) -> bool {
        self.runner_altitude <= 0.0
    }

    fn launch_jump(&mut self) {
        if self.is_grounded() {
            self.runner_altitude = 0.0;
            self.runner_velocity = (INITIAL_JUMP_VELOCITY - self.current_speed / 10.0).max(6.5);
            self.speed_drop = false;
            self.pending_jump = false;
        }
    }

    fn update_runner(&mut self) {
        if !self.is_grounded() || self.runner_velocity > 0.0 {
            self.runner_altitude += self.runner_velocity;

            let gravity = if self.speed_drop {
                GRAVITY * SPEED_DROP_COEFFICIENT
            } else if !self.jump_held
                && self.runner_velocity > 0.0
                && self.runner_altitude >= MIN_JUMP_HEIGHT
            {
                GRAVITY * 1.7
            } else {
                GRAVITY
            };

            self.runner_velocity -= gravity;
            if self.runner_altitude <= 0.0 {
                self.runner_altitude = 0.0;
                self.runner_velocity = 0.0;
                self.speed_drop = false;
            }
        } else {
            self.runner_altitude = 0.0;
            self.runner_velocity = 0.0;
        }

        if self.duck_held && !self.is_grounded() {
            self.speed_drop = true;
        }
    }

    fn decay_duck_hold(&mut self) {
        if !self.duck_held {
            return;
        }

        if self.duck_hold_steps > 0 {
            self.duck_hold_steps -= 1;
        }

        if self.duck_hold_steps == 0 {
            self.duck_held = false;
            self.speed_drop = false;
        }
    }

    fn update_obstacles(&mut self) {
        for obstacle in &mut self.obstacles {
            obstacle.x -= self.current_speed;
        }
        self.obstacles
            .retain(|obstacle| obstacle.x + obstacle.kind.logical_width() > 0.0);

        if self.next_spawn_in_steps > 0 {
            self.next_spawn_in_steps -= 1;
        }
        if self.next_spawn_in_steps == 0 {
            self.spawn_obstacle_group();
        }
    }

    fn spawn_obstacle_group(&mut self) {
        let kind = self.pick_obstacle_kind();
        let group_count = if kind.is_cactus() {
            let max_group = if self.current_speed >= 10.0 {
                3
            } else if self.current_speed >= 7.0 {
                2
            } else {
                1
            };
            1 + (self.next_rand() as usize % max_group)
        } else {
            1
        };

        let start_x = LOGICAL_WIDTH - kind.logical_width() - UNITS_PER_CELL;
        let mut x = start_x;
        for _ in 0..group_count {
            let altitude = self.obstacle_altitude(kind);
            self.obstacles.push(DinoObstacle { x, altitude, kind });
            x += kind.logical_width() + 18.0 + (self.next_rand() % 10) as f32;
        }

        let span_width = x - start_x;
        self.next_spawn_in_steps = self.compute_gap_steps(span_width.max(kind.logical_width()));
        if self.last_spawn_kind == Some(kind) {
            self.repeated_spawn_count = self.repeated_spawn_count.saturating_add(1);
        } else {
            self.repeated_spawn_count = 1;
            self.last_spawn_kind = Some(kind);
        }
    }

    fn pick_obstacle_kind(&mut self) -> DinoObstacleKind {
        let mut choices = vec![DinoObstacleKind::SmallCactus, DinoObstacleKind::LargeCactus];
        if self.current_speed >= 8.5 {
            choices.push(DinoObstacleKind::Pterodactyl);
        }

        for _ in 0..6 {
            let kind = choices[self.next_rand() as usize % choices.len()];
            if self.last_spawn_kind == Some(kind) && self.repeated_spawn_count >= 2 {
                continue;
            }
            return kind;
        }

        if self.last_spawn_kind == Some(DinoObstacleKind::SmallCactus) {
            DinoObstacleKind::LargeCactus
        } else {
            DinoObstacleKind::SmallCactus
        }
    }

    fn obstacle_altitude(&mut self, kind: DinoObstacleKind) -> f32 {
        match kind {
            DinoObstacleKind::SmallCactus | DinoObstacleKind::LargeCactus => 0.0,
            DinoObstacleKind::Pterodactyl => {
                let altitudes = [0.0, 24.0, 52.0];
                altitudes[self.next_rand() as usize % altitudes.len()]
            }
        }
    }

    fn compute_gap_steps(&mut self, obstacle_width: f32) -> u16 {
        let min_gap = obstacle_width * self.current_speed + MIN_OBSTACLE_GAP;
        let max_gap = min_gap * MAX_OBSTACLE_GAP_COEFFICIENT;
        let gap = min_gap + (self.next_rand() as f32 / u32::MAX as f32) * (max_gap - min_gap);
        ((gap / self.current_speed).round() as u16).max(18)
    }
}
