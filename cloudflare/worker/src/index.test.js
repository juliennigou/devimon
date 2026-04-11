import test from "node:test";
import assert from "node:assert/strict";

import {
  computeAcceptedRankedProgression,
  extractBearerToken,
  evaluateSuspiciousSync,
  maxXpGainSince,
  normalizeProfileStage,
  normalizeSeverity,
  parseSuspiciousSyncQuery,
  progressionFromTotalXp,
  stageForLevel,
  validateProfileSnapshot,
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

test("extractBearerToken parses bearer auth safely", () => {
  assert.equal(extractBearerToken("Bearer secret-token"), "secret-token");
  assert.equal(extractBearerToken("Basic abc"), null);
  assert.equal(extractBearerToken(null), null);
});

test("parseSuspiciousSyncQuery normalizes filters and caps limit", () => {
  const request = {
    url: "https://example.com/api/admin/suspicious-syncs?limit=999&account_id=acc_123&severity=HIGH",
  };

  assert.deepEqual(parseSuspiciousSyncQuery(request), {
    limit: 100,
    accountId: "acc_123",
    severity: "high",
  });
});

test("normalizeSeverity rejects unsupported values", () => {
  assert.throws(() => normalizeSeverity("critical"), /severity must be one of: warn, high/);
});

test("validateProfileSnapshot accepts inconsistent local progression fields", () => {
  const snapshot = validateProfileSnapshot({
    name: "Embit",
    level: 99,
    xp: 999,
    total_xp: 1,
    stage: "Baby",
    hunger: 80,
    energy: 75,
    mood: 90,
    last_active_at: "2026-04-11T22:00:00.000Z",
  });

  assert.equal(snapshot.level, 99);
  assert.equal(snapshot.xp, 999);
  assert.equal(snapshot.total_xp, 1);
  assert.equal(snapshot.stage, "Baby");
});

test("normalizeProfileStage still rejects invalid profile stages", () => {
  assert.equal(normalizeProfileStage("Young"), "Young");
  assert.throws(() => normalizeProfileStage("Legendary"), /snapshot.stage is invalid/);
});
