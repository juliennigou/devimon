use super::domain::{DinoGamePhase, DinoGameSession};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DinoExitReason {
    GameOver,
}

pub struct DinoRunResult {
    pub duration_ms: u64,
    pub score: u32,
    pub xp_awarded: u32,
    pub is_record: bool,
    pub exit_reason: DinoExitReason,
}

pub fn crash(session: &mut DinoGameSession, best_time_ms: u64) -> Option<DinoRunResult> {
    if session.phase == DinoGamePhase::Crashed {
        return None;
    }

    session.phase = DinoGamePhase::Crashed;
    session.crash_elapsed_ms = 0;

    Some(DinoRunResult {
        duration_ms: session.elapsed_ms,
        score: session.score,
        xp_awarded: session.xp_awarded,
        is_record: session.elapsed_ms > best_time_ms,
        exit_reason: DinoExitReason::GameOver,
    })
}
