import test from "node:test";
import assert from "node:assert/strict";

import {
  computeAcceptedRankedProgression,
  determineVerificationState,
  extractBearerToken,
  evaluateSuspiciousSync,
  maxXpGainSince,
  normalizeSeverity,
  normalizeVerificationStatus,
  parseSuspiciousSyncQuery,
  progressionFromTotalXp,
  stageForLevel,
  validateProfileSnapshot,
} from "./index.js";

test("first sync seeds ranked progression from client total xp", () => {
  const syncedAt = "2026-04-11T22:00:00.000Z";
  const progression = computeAcceptedRankedProgression(null, 500, syncedAt, 250);

  assert.equal(progression.totalXp, 250);
  assert.equal(progression.acceptedDelta, 0);
  assert.equal(progression.requestedDelta, 500);
});

test("first sync with zero client xp starts at zero", () => {
  const syncedAt = "2026-04-11T22:00:00.000Z";
  const progression = computeAcceptedRankedProgression(null, 0, syncedAt, 0);

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

test("validateProfileSnapshot accepts profile-only snapshots", () => {
  const snapshot = validateProfileSnapshot({
    name: "Embit",
    hunger: 80,
    energy: 75,
    mood: 90,
    total_xp: 90,
    last_active_at: "2026-04-11T22:00:00.000Z",
  });

  assert.equal(snapshot.name, "Embit");
  assert.equal(snapshot.hunger, 80);
  assert.equal(snapshot.energy, 75);
  assert.equal(snapshot.mood, 90);
  assert.equal(snapshot.total_xp, 90);
});

test("validateProfileSnapshot rejects missing profile fields", () => {
  assert.throws(
    () =>
      validateProfileSnapshot({
        name: "Embit",
        hunger: 80,
        energy: 75,
        mood: 90,
        total_xp: 90,
      }),
    /snapshot.last_active_at is required/
  );
});

test("validateProfileSnapshot falls back for legacy clients without total_xp", () => {
  const snapshot = validateProfileSnapshot(
    {
      name: "Embit",
      hunger: 80,
      energy: 75,
      mood: 90,
      last_active_at: "2026-04-11T22:00:00.000Z",
    },
    120
  );

  assert.equal(snapshot.total_xp, 120);
});

test("determineVerificationState verifies when trusted progression covers cloud total", () => {
  const verification = determineVerificationState(
    null,
    progressionFromTotalXp(90),
    {
      ...progressionFromTotalXp(90),
      acceptedDelta: 10,
      requestedDelta: 10,
      maxAcceptedDelta: 10,
    },
    [],
    "2026-04-12T10:00:00.000Z"
  );

  assert.deepEqual(verification, {
    status: "verified",
    verifiedAt: "2026-04-12T10:00:00.000Z",
    reason: "trusted_sync_history",
  });
});

test("determineVerificationState keeps imported higher cloud total unverified", () => {
  const verification = determineVerificationState(
    null,
    progressionFromTotalXp(250),
    {
      ...progressionFromTotalXp(90),
      acceptedDelta: 10,
      requestedDelta: 10,
      maxAcceptedDelta: 10,
    },
    [],
    "2026-04-12T10:00:00.000Z"
  );

  assert.deepEqual(verification, {
    status: "unverified",
    verifiedAt: null,
    reason: "awaiting_trusted_sync_history",
  });
});

test("determineVerificationState drops to unverified on suspicious syncs", () => {
  const verification = determineVerificationState(
    {
      verification_status: "verified",
      verified_at: "2026-04-11T10:00:00.000Z",
    },
    progressionFromTotalXp(90),
    {
      ...progressionFromTotalXp(90),
      acceptedDelta: 10,
      requestedDelta: 10,
      maxAcceptedDelta: 10,
    },
    [{ reason: "ranked_xp_capped", severity: "high" }],
    "2026-04-12T10:00:00.000Z"
  );

  assert.deepEqual(verification, {
    status: "unverified",
    verifiedAt: null,
    reason: "suspicious_activity",
  });
});

test("normalizeVerificationStatus falls back safely", () => {
  assert.equal(normalizeVerificationStatus("VERIFIED"), "verified");
  assert.equal(normalizeVerificationStatus("mystery"), "unverified");
  assert.equal(normalizeVerificationStatus(null), "unverified");
});
