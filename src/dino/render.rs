use crate::{display, monster::Monster, save::DINO_UNLOCK_TRIGGER_THRESHOLD_MS};

use super::{
    GROUND_HEIGHT_ROWS, LOGICAL_HEIGHT, LOGICAL_WIDTH, RUNNER_X,
    domain::{DinoGamePhase, DinoGameSession},
};

pub fn build_world(
    monster: &Monster,
    session: &DinoGameSession,
    width: usize,
    height: usize,
) -> Vec<String> {
    let world_width = width.max(20);
    let world_height = height.max(8);
    let mut canvas = vec![vec![' '; world_width]; world_height];
    let ground_y = world_height as i16 - GROUND_HEIGHT_ROWS as i16;
    let playable_rows = world_height.saturating_sub(GROUND_HEIGHT_ROWS).max(1);
    let units_per_row = LOGICAL_HEIGHT / playable_rows as f32;

    for x in 0..world_width {
        canvas[ground_y as usize][x] = if x % 6 == 0 { '┬' } else { '─' };
        canvas[(ground_y + 1) as usize][x] = if x % 2 == 0 { '·' } else { ' ' };
    }

    let runner = display::game_runner_sprite(monster, session.current_pose());
    let runner_y = runner_y(session, runner.len(), world_height);
    overlay_canvas(
        &mut canvas,
        project_x(RUNNER_X, world_width),
        runner_y,
        &runner,
    );

    for obstacle in &session.obstacles {
        let sprite: Vec<String> = obstacle
            .kind
            .sprite(session.frame)
            .iter()
            .map(|line| (*line).to_string())
            .collect();
        let obstacle_y =
            ground_y - sprite.len() as i16 - (obstacle.altitude / units_per_row).round() as i16;
        overlay_canvas(
            &mut canvas,
            project_x(obstacle.x, world_width),
            obstacle_y,
            &sprite,
        );
    }

    canvas
        .into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect()
}

pub fn status_text(best_time_ms: u64, session: &DinoGameSession) -> String {
    match session.phase {
        DinoGamePhase::Ready => "Press Space or ↑ to start. ↓ ducks and speed-drops.".to_string(),
        DinoGamePhase::Starting => "Get ready...".to_string(),
        DinoGamePhase::Running => {
            format!(
                "Score {}  ·  First record or {}s+ run queues an unlock trigger.",
                session.score,
                DINO_UNLOCK_TRIGGER_THRESHOLD_MS / 1000
            )
        }
        DinoGamePhase::Crashed if best_time_ms == session.elapsed_ms => {
            if session.can_restart() {
                "Crash. New record. Space or Enter to restart.".to_string()
            } else {
                format!(
                    "Crash. New record. Restart in {:.1}s.",
                    session.restart_remaining_ms() as f32 / 1000.0
                )
            }
        }
        DinoGamePhase::Crashed => {
            if session.can_restart() {
                "Crash. Space or Enter to restart.".to_string()
            } else {
                format!(
                    "Crash. Restart in {:.1}s.",
                    session.restart_remaining_ms() as f32 / 1000.0
                )
            }
        }
        DinoGamePhase::Paused => "Paused. Press Enter to continue.".to_string(),
        DinoGamePhase::Exiting => "Leaving Dino Run.".to_string(),
    }
}

pub fn format_duration_ms(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let tenths = (ms % 1000) / 100;
    format!("{}.{:01}s", total_seconds, tenths)
}

fn overlay_canvas(canvas: &mut [Vec<char>], x: i16, y: i16, sprite: &[String]) {
    for (row_idx, row) in sprite.iter().enumerate() {
        let yy = y + row_idx as i16;
        if yy < 0 || yy >= canvas.len() as i16 {
            continue;
        }
        for (col_idx, ch) in row.chars().enumerate() {
            let xx = x + col_idx as i16;
            if xx < 0 || xx >= canvas[yy as usize].len() as i16 || ch == ' ' {
                continue;
            }
            canvas[yy as usize][xx as usize] = ch;
        }
    }
}

pub(crate) fn runner_y(
    session: &DinoGameSession,
    runner_height: usize,
    world_height: usize,
) -> i16 {
    let ground_y = world_height as i16 - GROUND_HEIGHT_ROWS as i16;
    let playable_rows = world_height.saturating_sub(GROUND_HEIGHT_ROWS).max(1);
    let units_per_row = LOGICAL_HEIGHT / playable_rows as f32;
    let jump_rows = (session.runner_altitude / units_per_row).round() as i16;
    ground_y - runner_height as i16 - jump_rows
}

fn project_x(x: f32, world_width: usize) -> i16 {
    let max_x = world_width.saturating_sub(1) as f32;
    ((x / LOGICAL_WIDTH) * max_x).round() as i16
}
