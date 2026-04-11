import test from "node:test";
import assert from "node:assert/strict";

import {
  computeAcceptedRankedProgression,
  evaluateSuspiciousSync,
  maxXpGainSince,
  progressionFromTotalXp,
  stageForLevel,
} from "./index.js";

test("first sync starts ranked progression at zero", () => {
  const syncedAt = "2026-04-11T22:00:00.000Z";
  const progression = computeAcceptedRankedProgression(null, 500, syncedAt);

  assert.equal(progression.totalXp, 0);
  assert.equal(progression.level, 1);
  assert.equal(progression.xp, 0);
  assert.equal(progression.stage, "Baby");
  assert.equal(progression.acceptedDelta, 0);
});

test("ranked progression is capped by elapsed sync time", () => {
  const progression = computeAcceptedRankedProgression(
    {
      ranked_total_xp: 100,
      updated_at: "2026-04-11T22:00:00.000Z",
    },
    400,
    "2026-04-11T22:05:00.000Z"
  );

  assert.equal(maxXpGainSince("2026-04-11T22:00:00.000Z", "2026-04-11T22:05:00.000Z"), 60);
  assert.equal(progression.acceptedDelta, 60);
  assert.equal(progression.totalXp, 160);
});

test("ranked progression uses explicit ranked xp evidence, not client totals", () => {
  const progression = computeAcceptedRankedProgression(
    {
      ranked_total_xp: 100,
      updated_at: "2026-04-11T22:00:00.000Z",
    },
    20,
    "2026-04-11T22:05:00.000Z"
  );

  assert.equal(progression.requestedDelta, 20);
  assert.equal(progression.acceptedDelta, 20);
  assert.equal(progression.totalXp, 120);
});

test("progressionFromTotalXp derives stable level and xp", () => {
  const progression = progressionFromTotalXp(90);

  assert.equal(progression.level, 5);
  assert.equal(progression.xp, 0);
  assert.equal(progression.stage, "Young");
});

test("stageForLevel uses authoritative level thresholds", () => {
  assert.equal(stageForLevel(1), "Baby");
  assert.equal(stageForLevel(5), "Young");
  assert.equal(stageForLevel(15), "Evolved");
});

test("suspicious sync flags ranked xp with no elapsed time", () => {
  const progression = computeAcceptedRankedProgression(
    {
      ranked_total_xp: 100,
      updated_at: "2026-04-11T22:05:00.000Z",
    },
    50,
    "2026-04-11T22:05:00.000Z"
  );

  const findings = evaluateSuspiciousSync(50, progression);
  assert.deepEqual(findings, [
    {
      reason: "ranked_xp_without_elapsed_time",
      severity: "high",
    },
  ]);
});

test("suspicious sync flags capped and implausible ranked bursts", () => {
  const progression = computeAcceptedRankedProgression(
    {
      ranked_total_xp: 100,
      updated_at: "2026-04-11T22:00:00.000Z",
    },
    500,
    "2026-04-11T22:01:00.000Z"
  );

  const findings = evaluateSuspiciousSync(500, progression);
  assert.deepEqual(findings, [
    {
      reason: "ranked_xp_capped",
      severity: "high",
    },
    {
      reason: "ranked_xp_implausible_burst",
      severity: "high",
    },
  ]);
});
