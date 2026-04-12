# Audit Complete Checklist

This file records what still needs to happen before the Devimon audit can be considered complete.

## Remaining Work

- Strengthen the final evolution gate so it reflects real multi-day care quality.
- Improve XP semantics further so real coding is rewarded more reliably than noisy file activity.
- Finalize the cloud trust model so players clearly understand `Verified` versus `Unverified` leaderboard status.
- Add broader test coverage for:
  - sync edge cases
  - repeated capped sync behavior
  - pending ranked XP carryover
  - suspicious sync persistence paths
  - evolution timing behavior
- Perform one deployed end-to-end validation:
  - client sync
  - trusted cloud progression
  - leaderboard output
  - suspicious-sync debug endpoint

## Completion Standard

The audit can be called complete when:

- ranked truth is fully server-owned
- cheating paths are blocked or observable
- gameplay progression matches product intent
- tests protect the new rules
- deployed behavior has been validated end to end
