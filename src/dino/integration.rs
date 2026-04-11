use crate::save::{DinoGameProgress, DinoUnlockReason};

use super::domain::{DinoGamePhase, DinoGameSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DinoExitReason {
    GameOver,
}

#[derive(Debug)]
pub struct DinoRunResult {
    pub duration_ms: u64,
    pub score: u32,
    pub is_record: bool,
    pub unlock_reason: Option<DinoUnlockReason>,
    pub exit_reason: DinoExitReason,
}

pub fn crash(
    session: &mut DinoGameSession,
    progress: &mut DinoGameProgress,
) -> Option<DinoRunResult> {
    if session.phase == DinoGamePhase::Crashed {
        return None;
    }

    let duration_ms = session.elapsed_ms;
    let is_record = duration_ms > progress.best_time_ms;
    let unlock_reason = progress.register_run_completion(duration_ms);

    session.phase = DinoGamePhase::Crashed;
    session.crash_elapsed_ms = 0;

    Some(DinoRunResult {
        duration_ms,
        score: session.score,
        is_record,
        unlock_reason,
        exit_reason: DinoExitReason::GameOver,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_record_queues_unlock_trigger() {
        let mut session = DinoGameSession::new(1);
        session.elapsed_ms = 95_000;
        session.score = 42;
        let mut progress = DinoGameProgress::default();

        let result = crash(&mut session, &mut progress).expect("run should crash cleanly");

        assert!(result.is_record);
        assert_eq!(result.unlock_reason, Some(DinoUnlockReason::FirstRecord));
        assert_eq!(progress.best_time_ms, 95_000);
        assert_eq!(progress.pending_unlock_triggers, 1);
        assert!(progress.record_unlock_claimed);
        assert_eq!(session.phase, DinoGamePhase::Crashed);
    }

    #[test]
    fn long_non_record_run_queues_endurance_trigger() {
        let mut progress = DinoGameProgress::default();
        progress.best_time_ms = 100_000;
        progress.record_unlock_claimed = true;

        let mut session = DinoGameSession::new(1);
        session.elapsed_ms = 130_000;
        session.score = 99;

        let result = crash(&mut session, &mut progress).expect("run should crash cleanly");

        assert!(result.is_record);
        assert_eq!(result.unlock_reason, Some(DinoUnlockReason::Endurance));
        assert_eq!(progress.best_time_ms, 130_000);
        assert_eq!(progress.pending_unlock_triggers, 1);
    }
}
