use crate::monster::Monster;

use super::{
    RUNNER_X, UNITS_PER_CELL,
    domain::{DinoGameSession, DinoObstacle, DinoObstacleKind},
};

pub fn has_collision(monster: &Monster, session: &DinoGameSession) -> bool {
    let _ = monster;
    let runner_boxes = runner_hitboxes(session);

    session.obstacles.iter().any(|obstacle| {
        let obstacle_boxes = obstacle_hitboxes(*obstacle);
        runner_boxes.iter().any(|runner_box| {
            obstacle_boxes
                .iter()
                .any(|obstacle_box| rects_overlap(*runner_box, *obstacle_box))
        })
    })
}

fn runner_hitboxes(session: &DinoGameSession) -> Vec<(f32, f32, f32, f32)> {
    let base_y = session.runner_altitude;
    let rects = if session.is_ducking() {
        vec![(1.0, 0.6, 7.7, 2.0), (2.0, 2.0, 7.0, 3.1)]
    } else {
        vec![(2.0, 0.8, 7.2, 2.2), (1.2, 2.2, 7.4, 4.2)]
    };

    rects
        .into_iter()
        .map(|(x1, y1, x2, y2)| {
            (
                RUNNER_X + x1 * UNITS_PER_CELL,
                base_y + y1 * UNITS_PER_CELL,
                RUNNER_X + x2 * UNITS_PER_CELL,
                base_y + y2 * UNITS_PER_CELL,
            )
        })
        .collect()
}

fn obstacle_hitboxes(obstacle: DinoObstacle) -> Vec<(f32, f32, f32, f32)> {
    let rects = match obstacle.kind {
        DinoObstacleKind::SmallCactus => vec![(1.0, 0.0, 2.8, 4.0)],
        DinoObstacleKind::LargeCactus => vec![(0.8, 0.0, 4.1, 4.0)],
        DinoObstacleKind::Pterodactyl => vec![(0.8, 1.3, 5.2, 2.7), (1.8, 2.7, 4.0, 4.0)],
    };

    rects
        .into_iter()
        .map(|(x1, y1, x2, y2)| {
            (
                obstacle.x + x1 * UNITS_PER_CELL,
                obstacle.altitude + y1 * UNITS_PER_CELL,
                obstacle.x + x2 * UNITS_PER_CELL,
                obstacle.altitude + y2 * UNITS_PER_CELL,
            )
        })
        .collect()
}

fn rects_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let (ax1, ay1, ax2, ay2) = a;
    let (bx1, by1, bx2, by2) = b;
    ax1 <= bx2 && ax2 >= bx1 && ay1 <= by2 && ay2 >= by1
}
