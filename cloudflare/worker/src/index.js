const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, content-type",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

const ALLOWED_STAGES = new Set(["Baby", "Young", "Evolved"]);
const ALLOWED_VERIFICATION_STATUSES = new Set(["verified", "unverified"]);
const MAX_SYNC_LEVEL = 10000;
const XP_PER_MINUTE_CAP = 10;
const SYNC_XP_GRACE = 10;
const RANKED_MONSTER_COLUMNS = [
  { name: "ranked_level", ddl: "ALTER TABLE monsters ADD COLUMN ranked_level INTEGER NOT NULL DEFAULT 1 CHECK (ranked_level >= 1)" },
  { name: "ranked_xp", ddl: "ALTER TABLE monsters ADD COLUMN ranked_xp INTEGER NOT NULL DEFAULT 0 CHECK (ranked_xp >= 0)" },
  { name: "ranked_total_xp", ddl: "ALTER TABLE monsters ADD COLUMN ranked_total_xp INTEGER NOT NULL DEFAULT 0 CHECK (ranked_total_xp >= 0)" },
  {
    name: "ranked_stage",
    ddl: "ALTER TABLE monsters ADD COLUMN ranked_stage TEXT NOT NULL DEFAULT 'Baby' CHECK (ranked_stage IN ('Baby', 'Young', 'Evolved'))",
  },
];
const RANKED_MONSTER_INDEX_DDL =
  `CREATE INDEX IF NOT EXISTS idx_monsters_ranked_total_xp
    ON monsters (ranked_total_xp DESC, ranked_level DESC, updated_at DESC)`;
const VERIFICATION_MONSTER_COLUMNS = [
  {
    name: "verification_status",
    ddl: "ALTER TABLE monsters ADD COLUMN verification_status TEXT NOT NULL DEFAULT 'unverified' CHECK (verification_status IN ('verified', 'unverified'))",
  },
  {
    name: "verified_at",
    ddl: "ALTER TABLE monsters ADD COLUMN verified_at TEXT",
  },
  {
    name: "verification_reason",
    ddl: "ALTER TABLE monsters ADD COLUMN verification_reason TEXT",
  },
];
const VERIFICATION_MONSTER_INDEX_DDL =
  `CREATE INDEX IF NOT EXISTS idx_monsters_verification_status
    ON monsters (verification_status, updated_at DESC)`;

export default {
  async fetch(request, env) {
    try {
      if (request.method === "OPTIONS") {
        return new Response(null, { status: 204, headers: corsHeaders });
      }

      const url = new URL(request.url);
      const { pathname } = url;

      if (pathname === "/api/auth/github/device/start" && request.method === "POST") {
        return await startGitHubDeviceFlow(env);
      }

      if (pathname === "/api/auth/github/device/poll" && request.method === "POST") {
        return await pollGitHubDeviceFlow(request, env);
      }

      if (pathname === "/api/me" && request.method === "GET") {
        const session = await requireSession(request, env);
        return await handleMe(session, env);
      }

      if (pathname === "/api/sync" && request.method === "POST") {
        const session = await requireSession(request, env);
        return await handleSync(request, env, session);
      }

      if (pathname === "/api/monster/link" && request.method === "POST") {
        const session = await requireSession(request, env);
        return await handleSync(request, env, session);
      }

      if (pathname === "/api/leaderboard" && request.method === "GET") {
        return await handleLeaderboard(request, env);
      }

      if (pathname === "/api/admin/suspicious-syncs" && request.method === "GET") {
        requireAdminToken(request, env);
        return await handleAdminSuspiciousSyncs(request, env);
      }

      return json({ error: "not_found" }, 404);
    } catch (error) {
      if (error instanceof HttpError) {
        return json({ error: error.message }, error.status);
      }

      return json({ error: error.message || "internal_server_error" }, 500);
    }
  },
};

class HttpError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      ...corsHeaders,
    },
  });
}

function nowIso() {
  return new Date().toISOString();
}

function requireEnv(env, key) {
  if (!env[key]) {
    throw new HttpError(500, `missing worker secret: ${key}`);
  }
  return env[key];
}

function requireAdminToken(request, env) {
  const expectedToken = requireEnv(env, "ADMIN_DEBUG_TOKEN");
  const providedToken =
    request.headers.get("x-admin-token") ||
    extractBearerToken(request.headers.get("authorization"));

  if (!providedToken || providedToken !== expectedToken) {
    throw new HttpError(401, "invalid admin token");
  }
}

function extractBearerToken(authHeader) {
  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    return null;
  }

  const token = authHeader.slice("Bearer ".length).trim();
  return token || null;
}

async function readJson(request) {
  try {
    return await request.json();
  } catch {
    throw new HttpError(400, "request body must be valid JSON");
  }
}

async function requireSession(request, env) {
  const token = extractBearerToken(request.headers.get("authorization"));
  if (!token) {
    throw new HttpError(401, "missing bearer token");
  }

  const session = await env.DB.prepare(
    `SELECT s.session_token, s.account_id, a.username
       FROM sessions s
       JOIN accounts a ON a.account_id = s.account_id
      WHERE s.session_token = ?
        AND s.expires_at > ?`
  )
    .bind(token, nowIso())
    .first();

  if (!session) {
    throw new HttpError(401, "invalid or expired session");
  }

  return session;
}

async function startGitHubDeviceFlow(env) {
  const clientId = requireEnv(env, "GITHUB_CLIENT_ID");
  const response = await fetch("https://github.com/login/device/code", {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/x-www-form-urlencoded",
      "User-Agent": "devimon-worker",
    },
    body: new URLSearchParams({
      client_id: clientId,
      scope: "read:user",
    }),
  });

  const payload = await response.json();
  if (!response.ok) {
    throw new HttpError(502, payload.error_description || "failed to start GitHub device flow");
  }

  const loginId = crypto.randomUUID();
  const createdAt = nowIso();
  const expiresAt = new Date(Date.now() + payload.expires_in * 1000).toISOString();
  await env.DB.prepare(
    `INSERT INTO pending_device_logins (
        login_id, device_code, user_code, verification_uri,
        interval_seconds, expires_at, status, created_at, updated_at
      )
      VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?)`
  )
    .bind(
      loginId,
      payload.device_code,
      payload.user_code,
      payload.verification_uri,
      payload.interval,
      expiresAt,
      createdAt,
      createdAt
    )
    .run();

  return json({
    login_id: loginId,
    user_code: payload.user_code,
    verification_uri: payload.verification_uri,
    interval_seconds: payload.interval,
    expires_at: expiresAt,
  });
}

async function pollGitHubDeviceFlow(request, env) {
  const body = await readJson(request);
  const loginId = body.login_id;
  if (!loginId) {
    throw new HttpError(400, "login_id is required");
  }

  const pending = await env.DB.prepare(
    `SELECT *
       FROM pending_device_logins
      WHERE login_id = ?`
  )
    .bind(loginId)
    .first();

  if (!pending) {
    throw new HttpError(404, "unknown login session");
  }

  if (pending.status === "complete") {
    return json({
      status: "complete",
      account: {
        account_id: pending.account_id,
        username: pending.username,
        session_token: pending.session_token,
      },
    });
  }

  if (pending.expires_at <= nowIso()) {
    await env.DB.prepare(
      `UPDATE pending_device_logins
          SET status = 'expired', updated_at = ?
        WHERE login_id = ?`
    )
      .bind(nowIso(), loginId)
      .run();

    return json({
      status: "expired",
      message: "device authorization expired; run `devimon login` again",
    });
  }

  const tokenPayload = await fetchGitHubAccessToken(env, pending.device_code);
  if (tokenPayload.error === "authorization_pending") {
    return json({
      status: "pending",
      interval_seconds: pending.interval_seconds,
    });
  }

  if (tokenPayload.error === "slow_down") {
    const nextInterval = Number(pending.interval_seconds) + 5;
    await env.DB.prepare(
      `UPDATE pending_device_logins
          SET interval_seconds = ?, updated_at = ?
        WHERE login_id = ?`
    )
      .bind(nextInterval, nowIso(), loginId)
      .run();
    return json({
      status: "pending",
      interval_seconds: nextInterval,
      message: "GitHub asked the client to slow down polling.",
    });
  }

  if (tokenPayload.error === "expired_token") {
    await env.DB.prepare(
      `UPDATE pending_device_logins
          SET status = 'expired', updated_at = ?
        WHERE login_id = ?`
    )
      .bind(nowIso(), loginId)
      .run();
    return json({
      status: "expired",
      message: "device authorization expired; run `devimon login` again",
    });
  }

  if (tokenPayload.error === "access_denied") {
    await env.DB.prepare(
      `UPDATE pending_device_logins
          SET status = 'denied', updated_at = ?
        WHERE login_id = ?`
    )
      .bind(nowIso(), loginId)
      .run();
    return json({
      status: "denied",
      message: "GitHub denied the login request.",
    });
  }

  if (!tokenPayload.access_token) {
    throw new HttpError(502, "GitHub device flow returned an unexpected response");
  }

  const githubUser = await fetchGitHubUser(tokenPayload.access_token);
  const account = await upsertAccount(env, githubUser);
  const sessionToken = crypto.randomUUID();
  const createdAt = nowIso();
  const expiresAt = new Date(Date.now() + 180 * 24 * 3600 * 1000).toISOString();

  await env.DB.prepare(
    `INSERT INTO sessions (session_token, account_id, expires_at, created_at)
      VALUES (?, ?, ?, ?)`
  )
    .bind(sessionToken, account.account_id, expiresAt, createdAt)
    .run();

  await env.DB.prepare(
    `UPDATE pending_device_logins
        SET status = 'complete',
            account_id = ?,
            username = ?,
            session_token = ?,
            updated_at = ?
      WHERE login_id = ?`
  )
    .bind(account.account_id, account.username, sessionToken, createdAt, loginId)
    .run();

  return json({
    status: "complete",
    account: {
      account_id: account.account_id,
      username: account.username,
      session_token: sessionToken,
    },
  });
}

async function fetchGitHubAccessToken(env, deviceCode) {
  const clientId = requireEnv(env, "GITHUB_CLIENT_ID");
  const clientSecret = requireEnv(env, "GITHUB_CLIENT_SECRET");
  const response = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/x-www-form-urlencoded",
      "User-Agent": "devimon-worker",
    },
    body: new URLSearchParams({
      client_id: clientId,
      client_secret: clientSecret,
      device_code: deviceCode,
      grant_type: "urn:ietf:params:oauth:grant-type:device_code",
    }),
  });

  return await response.json();
}

async function fetchGitHubUser(accessToken) {
  const response = await fetch("https://api.github.com/user", {
    headers: {
      Accept: "application/vnd.github+json",
      Authorization: `Bearer ${accessToken}`,
      "User-Agent": "devimon-worker",
    },
  });

  if (!response.ok) {
    throw new HttpError(502, "failed to fetch GitHub profile");
  }

  return await response.json();
}

async function upsertAccount(env, githubUser) {
  const existing = await env.DB.prepare(
    `SELECT account_id
       FROM accounts
      WHERE github_user_id = ?`
  )
    .bind(githubUser.id)
    .first();

  const accountId = existing?.account_id || crypto.randomUUID();
  const timestamp = nowIso();

  await env.DB.prepare(
    `INSERT INTO accounts (account_id, github_user_id, username, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?)
      ON CONFLICT(github_user_id)
      DO UPDATE SET
        username = excluded.username,
        updated_at = excluded.updated_at`
  )
    .bind(accountId, githubUser.id, githubUser.login, timestamp, timestamp)
    .run();

  return {
    account_id: accountId,
    username: githubUser.login,
  };
}

async function handleMe(session, env) {
  const monster = await env.DB.prepare(
    `SELECT monster_id
       FROM monsters
      WHERE account_id = ?`
  )
    .bind(session.account_id)
    .first();

  return json({
    account_id: session.account_id,
    username: session.username,
    monster_id: monster?.monster_id || null,
  });
}

async function handleSync(request, env, session) {
  await ensureRankedMonsterColumns(env);
  await ensureVerificationMonsterColumns(env);
  await ensureSuspiciousSyncsTable(env);

  const body = await readJson(request);
  if (typeof body.device_id !== "string" || !body.device_id.trim()) {
    throw new HttpError(400, "device_id is required");
  }
  if (!body.snapshot || typeof body.snapshot !== "object" || Array.isArray(body.snapshot)) {
    throw new HttpError(400, "snapshot is required");
  }
  if (
    body.ranked_xp_delta !== undefined &&
    (!Number.isInteger(body.ranked_xp_delta) || body.ranked_xp_delta < 0)
  ) {
    throw new HttpError(400, "ranked_xp_delta must be a non-negative integer");
  }

  const deviceId = body.device_id.trim();
  const clientMonsterId =
    typeof body.monster_id === "string" && body.monster_id.trim()
      ? body.monster_id.trim()
      : null;
  const rankedXpDelta = Number(body.ranked_xp_delta || 0);
  const syncedAt = nowIso();

  await env.DB.prepare(
    `INSERT INTO devices (device_id, account_id, last_seen_at, created_at)
      VALUES (?, ?, ?, ?)
      ON CONFLICT(device_id)
      DO UPDATE SET
        account_id = excluded.account_id,
        last_seen_at = excluded.last_seen_at`
  )
    .bind(deviceId, session.account_id, syncedAt, syncedAt)
    .run();

  const existing = await env.DB.prepare(
    `SELECT monster_id, ranked_total_xp, updated_at, verification_status, verified_at
       FROM monsters
      WHERE account_id = ?`
  )
    .bind(session.account_id)
    .first();

  // Monster ownership is server-side: client-supplied IDs are ignored here.
  const monsterId = existing?.monster_id || crypto.randomUUID();
  const rankedProgression = computeAcceptedRankedProgression(existing, rankedXpDelta, syncedAt);
  const snapshot = validateProfileSnapshot(body.snapshot, rankedProgression.totalXp);
  const suspiciousFindings = evaluateSuspiciousSync(rankedXpDelta, rankedProgression);
  const verification = determineVerificationState(
    existing,
    suspiciousFindings,
    syncedAt
  );

  await env.DB.prepare(
    `INSERT INTO monsters (
        monster_id, account_id, name, level, xp, total_xp, stage,
        ranked_level, ranked_xp, ranked_total_xp, ranked_stage,
        verification_status, verified_at, verification_reason,
        hunger, energy, mood, last_active_at, updated_at
      )
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(account_id)
      DO UPDATE SET
        name = excluded.name,
        level = excluded.level,
        xp = excluded.xp,
        total_xp = excluded.total_xp,
        stage = excluded.stage,
        ranked_level = excluded.ranked_level,
        ranked_xp = excluded.ranked_xp,
        ranked_total_xp = excluded.ranked_total_xp,
        ranked_stage = excluded.ranked_stage,
        verification_status = excluded.verification_status,
        verified_at = excluded.verified_at,
        verification_reason = excluded.verification_reason,
        hunger = excluded.hunger,
        energy = excluded.energy,
        mood = excluded.mood,
        last_active_at = excluded.last_active_at,
        updated_at = excluded.updated_at`
  )
    .bind(
      monsterId,
      session.account_id,
      snapshot.name,
      rankedProgression.level,
      rankedProgression.xp,
      rankedProgression.totalXp,
      rankedProgression.stage,
      rankedProgression.level,
      rankedProgression.xp,
      rankedProgression.totalXp,
      rankedProgression.stage,
      verification.status,
      verification.verifiedAt,
      verification.reason,
      snapshot.hunger,
      snapshot.energy,
      snapshot.mood,
      snapshot.last_active_at,
      syncedAt
    )
    .run();

  const canonicalMonster = await env.DB.prepare(
    `SELECT monster_id
       FROM monsters
      WHERE account_id = ?`
  )
    .bind(session.account_id)
    .first();

  if (!canonicalMonster?.monster_id) {
    throw new HttpError(500, "failed to resolve monster ownership");
  }

  await env.DB.prepare(
    `INSERT INTO sync_history (id, monster_id, device_id, received_at, payload_json)
      VALUES (?, ?, ?, ?, ?)`
  )
    .bind(
      crypto.randomUUID(),
      canonicalMonster.monster_id,
      deviceId,
      syncedAt,
      JSON.stringify({
        device_id: deviceId,
        client_monster_id: clientMonsterId,
        resolved_monster_id: canonicalMonster.monster_id,
        ranked_progression: rankedProgression,
        verification,
        ranked_xp_delta: rankedXpDelta,
        snapshot,
      })
    )
    .run();

  if (suspiciousFindings.length > 0) {
    await persistSuspiciousSyncs(
      env,
      session.account_id,
      canonicalMonster.monster_id,
      deviceId,
      suspiciousFindings,
      {
        client_monster_id: clientMonsterId,
        ranked_xp_delta: rankedXpDelta,
        ranked_progression: rankedProgression,
        verification,
        snapshot,
      },
      syncedAt
    );
  }

  let officialRank = null;
  if (verification.status === "verified") {
    const rankRow = await env.DB.prepare(
      `SELECT COUNT(*) + 1 AS rank
         FROM monsters
        WHERE verification_status = 'verified'
          AND (
            ranked_total_xp > ?
            OR (ranked_total_xp = ? AND ranked_level > ?)
            OR (ranked_total_xp = ? AND ranked_level = ? AND updated_at > ?)
          )`
    )
      .bind(
        rankedProgression.totalXp,
        rankedProgression.totalXp,
        rankedProgression.level,
        rankedProgression.totalXp,
        rankedProgression.level,
        syncedAt
      )
      .first();
    officialRank = rankRow?.rank ? Number(rankRow.rank) : null;
  }

  return json({
    monster_id: canonicalMonster.monster_id,
    synced_at: syncedAt,
    verification_status: verification.status,
    official_rank: officialRank,
    leaderboard_rank: officialRank,
    cloud_total_xp: rankedProgression.totalXp,
    cloud_level: rankedProgression.level,
    cloud_stage: rankedProgression.stage,
    accepted_xp_delta: rankedProgression.acceptedDelta,
    requested_xp_delta: rankedProgression.requestedDelta,
    max_accepted_xp_delta: rankedProgression.maxAcceptedDelta,
  });
}

function validateProfileSnapshot(snapshot, fallbackTotalXp = 0) {
  const requiredStrings = ["name", "last_active_at"];
  for (const key of requiredStrings) {
    if (typeof snapshot[key] !== "string" || !snapshot[key].trim()) {
      throw new HttpError(400, `snapshot.${key} is required`);
    }
  }

  const numericKeys = ["hunger", "energy", "mood"];
  for (const key of numericKeys) {
    if (typeof snapshot[key] !== "number" || !Number.isFinite(snapshot[key])) {
      throw new HttpError(400, `snapshot.${key} must be a number`);
    }
  }
  if (
    snapshot.total_xp !== undefined &&
    (!Number.isInteger(snapshot.total_xp) ||
      snapshot.total_xp < 0 ||
      snapshot.total_xp > totalXpForLevel(MAX_SYNC_LEVEL))
  ) {
    throw new HttpError(400, "snapshot.total_xp must be a non-negative integer");
  }

  // Snapshot fields are profile-only and never drive ranked truth.
  // Snapshot progression is display-only. Ranked progression is derived
  // exclusively from trusted ranked XP evidence.
  const lastActiveAt = parseIsoTimestamp(snapshot.last_active_at, "snapshot.last_active_at");

  return {
    name: snapshot.name.trim().slice(0, 40),
    hunger: clamp(snapshot.hunger, 0, 100),
    energy: clamp(snapshot.energy, 0, 100),
    mood: clamp(snapshot.mood, 0, 100),
    total_xp: Number.isInteger(snapshot.total_xp) ? snapshot.total_xp : fallbackTotalXp,
    last_active_at: lastActiveAt,
  };
}

function stageForLevel(level) {
  if (level >= 15) {
    return "Evolved";
  }
  if (level >= 5) {
    return "Young";
  }
  return "Baby";
}

function totalXpForLevel(level) {
  let total = 0;
  for (let currentLevel = 1; currentLevel < level; currentLevel += 1) {
    total += 10 + currentLevel * 5;
  }
  return total;
}

function progressionFromTotalXp(totalXp) {
  let level = 1;
  let remaining = totalXp;

  while (remaining >= 10 + level * 5) {
    remaining -= 10 + level * 5;
    level += 1;
  }

  return {
    level,
    xp: remaining,
    totalXp,
    stage: stageForLevel(level),
  };
}

function computeAcceptedRankedProgression(existing, requestedXpDelta, syncedAt) {
  // First sync: start at 0. All XP must be earned through verified syncs.
  if (!existing) {
    return {
      ...progressionFromTotalXp(0),
      acceptedDelta: 0,
      requestedDelta: Math.max(0, requestedXpDelta),
      maxAcceptedDelta: 0,
    };
  }

  const previousTotalXp = Number(existing.ranked_total_xp || 0);
  const requestedDelta = Math.max(0, requestedXpDelta);
  const maxAcceptedDelta = maxXpGainSince(existing.updated_at, syncedAt);
  const acceptedDelta = Math.min(requestedDelta, maxAcceptedDelta);
  const trustedTotalXp = previousTotalXp + acceptedDelta;

  return {
    ...progressionFromTotalXp(trustedTotalXp),
    acceptedDelta,
    requestedDelta,
    maxAcceptedDelta,
  };
}

function determineVerificationState(
  existing,
  suspiciousFindings,
  syncedAt
) {
  const existingStatus = normalizeVerificationStatus(existing?.verification_status);
  const existingVerifiedAt =
    typeof existing?.verified_at === "string" && existing.verified_at.trim()
      ? existing.verified_at
      : null;

  if (suspiciousFindings.length > 0) {
    return {
      status: "unverified",
      verifiedAt: null,
      reason: "suspicious_activity",
    };
  }

  return {
    status: "verified",
    verifiedAt:
      existingStatus === "verified" && existingVerifiedAt ? existingVerifiedAt : syncedAt,
    reason: "trusted_sync_history",
  };
}

function evaluateSuspiciousSync(rankedXpDelta, rankedProgression) {
  const findings = [];
  if (rankedXpDelta <= 0) {
    return findings;
  }

  if (rankedProgression.maxAcceptedDelta === 0 && rankedProgression.requestedDelta > 0) {
    findings.push({
      reason: "ranked_xp_without_elapsed_time",
      severity: "high",
    });
  } else if (rankedProgression.requestedDelta > rankedProgression.maxAcceptedDelta) {
    const severity =
      rankedProgression.requestedDelta >= rankedProgression.maxAcceptedDelta * 3
        ? "high"
        : "warn";
    findings.push({
      reason: "ranked_xp_capped",
      severity,
    });
  }

  if (
    rankedProgression.maxAcceptedDelta <= XP_PER_MINUTE_CAP + SYNC_XP_GRACE &&
    rankedProgression.requestedDelta >= 250
  ) {
    findings.push({
      reason: "ranked_xp_implausible_burst",
      severity: "high",
    });
  }

  return findings;
}

function maxXpGainSince(previousUpdatedAt, syncedAt) {
  if (!previousUpdatedAt) {
    return 0;
  }

  const previous = Date.parse(previousUpdatedAt);
  const current = Date.parse(syncedAt);
  if (!Number.isFinite(previous) || !Number.isFinite(current) || current <= previous) {
    return 0;
  }

  const elapsedMinutes = Math.floor((current - previous) / 60_000);
  return elapsedMinutes * XP_PER_MINUTE_CAP + SYNC_XP_GRACE;
}

function parseIsoTimestamp(value, fieldName) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    throw new HttpError(400, `${fieldName} must be a valid ISO-8601 timestamp`);
  }
  return parsed.toISOString();
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

async function handleLeaderboard(request, env) {
  await ensureRankedMonsterColumns(env);
  await ensureVerificationMonsterColumns(env);

  const url = new URL(request.url);
  const requested = Number(url.searchParams.get("limit") || 20);
  const limit = Number.isFinite(requested)
    ? Math.min(Math.max(Math.floor(requested), 1), 100)
    : 20;
  const verifiedOnly = ["1", "true", "yes"].includes(
    (url.searchParams.get("verified_only") || "").trim().toLowerCase()
  );

  let sql =
    `SELECT m.monster_id, m.name,
            m.ranked_level, m.ranked_total_xp, m.ranked_stage,
            m.verification_status, m.verified_at,
            m.updated_at, m.last_active_at, a.username
       FROM monsters m
       JOIN accounts a ON m.account_id = a.account_id`;
  if (verifiedOnly) {
    sql += " WHERE m.verification_status = 'verified'";
  }
  sql += `
      ORDER BY
        CASE WHEN m.verification_status = 'verified' THEN 0 ELSE 1 END ASC,
        m.ranked_total_xp DESC,
        m.ranked_level DESC,
        m.updated_at DESC
      LIMIT ?`;

  const rows = await env.DB.prepare(sql).bind(limit).all();

  let nextOfficialRank = 1;
  let verifiedCount = 0;
  const monsters = (rows.results || []).map((row) => {
    const verificationStatus = normalizeVerificationStatus(row.verification_status);
    const officialRank = verificationStatus === "verified" ? nextOfficialRank++ : null;
    if (verificationStatus === "verified") {
      verifiedCount += 1;
    }
    return {
      rank: officialRank,
      official_rank: officialRank,
      monster_id: row.monster_id,
      name: row.name,
      github_username: row.username,
      level: Number(row.ranked_level),
      total_xp: Number(row.ranked_total_xp),
      stage: row.ranked_stage,
      verification_status: verificationStatus,
      verified_at: row.verified_at,
      updated_at: row.updated_at,
      last_active_at: row.last_active_at,
    };
  });

  return json({
    generated_at: nowIso(),
    verified_only: verifiedOnly,
    verified_count: verifiedCount,
    monster_count: monsters.length,
    monsters,
  });
}

async function handleAdminSuspiciousSyncs(request, env) {
  await ensureSuspiciousSyncsTable(env);

  const { limit, accountId, severity } = parseSuspiciousSyncQuery(request);
  let sql = `SELECT id, account_id, monster_id, device_id, reason, severity,
                    requested_ranked_xp_delta, accepted_ranked_xp_delta,
                    max_accepted_ranked_xp_delta, trusted_total_xp_after,
                    detected_at
               FROM suspicious_syncs`;
  const clauses = [];
  const bindings = [];

  if (accountId) {
    clauses.push("account_id = ?");
    bindings.push(accountId);
  }
  if (severity) {
    clauses.push("severity = ?");
    bindings.push(severity);
  }
  if (clauses.length > 0) {
    sql += ` WHERE ${clauses.join(" AND ")}`;
  }
  sql += " ORDER BY detected_at DESC LIMIT ?";
  bindings.push(limit);

  const rows = await env.DB.prepare(sql).bind(...bindings).all();
  const suspiciousSyncs = (rows.results || []).map((row) => ({
    id: row.id,
    account_id: row.account_id,
    monster_id: row.monster_id,
    device_id: row.device_id,
    reason: row.reason,
    severity: row.severity,
    requested_ranked_xp_delta: Number(row.requested_ranked_xp_delta),
    accepted_ranked_xp_delta: Number(row.accepted_ranked_xp_delta),
    max_accepted_ranked_xp_delta: Number(row.max_accepted_ranked_xp_delta),
    trusted_total_xp_after: Number(row.trusted_total_xp_after),
    detected_at: row.detected_at,
  }));

  return json({
    generated_at: nowIso(),
    filters: {
      limit,
      account_id: accountId,
      severity,
    },
    suspicious_syncs: suspiciousSyncs,
  });
}

function parseSuspiciousSyncQuery(request) {
  const url = new URL(request.url);
  const requestedLimit = Number(url.searchParams.get("limit") || 20);
  const limit = Number.isFinite(requestedLimit)
    ? Math.min(Math.max(Math.floor(requestedLimit), 1), 100)
    : 20;

  const accountId = url.searchParams.get("account_id")?.trim() || null;
  const severity = normalizeSeverity(url.searchParams.get("severity"));

  return {
    limit,
    accountId,
    severity,
  };
}

function normalizeSeverity(value) {
  if (!value) {
    return null;
  }

  const normalized = value.trim().toLowerCase();
  if (normalized === "warn" || normalized === "high") {
    return normalized;
  }

  throw new HttpError(400, "severity must be one of: warn, high");
}

function normalizeVerificationStatus(value) {
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    if (ALLOWED_VERIFICATION_STATUSES.has(normalized)) {
      return normalized;
    }
  }
  return "unverified";
}

async function ensureRankedMonsterColumns(env) {
  const result = await env.DB.prepare("PRAGMA table_info(monsters)").all();
  const existingColumns = new Set((result.results || []).map((column) => column.name));

  for (const column of RANKED_MONSTER_COLUMNS) {
    if (existingColumns.has(column.name)) {
      continue;
    }
    await env.DB.prepare(column.ddl).run();
    existingColumns.add(column.name);
  }

  await env.DB.prepare(RANKED_MONSTER_INDEX_DDL).run();
}

async function ensureVerificationMonsterColumns(env) {
  const result = await env.DB.prepare("PRAGMA table_info(monsters)").all();
  const existingColumns = new Set((result.results || []).map((column) => column.name));

  for (const column of VERIFICATION_MONSTER_COLUMNS) {
    if (existingColumns.has(column.name)) {
      continue;
    }
    await env.DB.prepare(column.ddl).run();
    existingColumns.add(column.name);
  }

  await env.DB.prepare(VERIFICATION_MONSTER_INDEX_DDL).run();
}

async function ensureSuspiciousSyncsTable(env) {
  await env.DB.prepare(
    `CREATE TABLE IF NOT EXISTS suspicious_syncs (
      id TEXT PRIMARY KEY,
      account_id TEXT NOT NULL,
      monster_id TEXT,
      device_id TEXT NOT NULL,
      reason TEXT NOT NULL,
      severity TEXT NOT NULL,
      requested_ranked_xp_delta INTEGER NOT NULL,
      accepted_ranked_xp_delta INTEGER NOT NULL,
      max_accepted_ranked_xp_delta INTEGER NOT NULL,
      trusted_total_xp_after INTEGER NOT NULL,
      payload_json TEXT NOT NULL,
      detected_at TEXT NOT NULL
    )`
  ).run();

  await env.DB.prepare(
    `CREATE INDEX IF NOT EXISTS idx_suspicious_syncs_account_detected
      ON suspicious_syncs (account_id, detected_at DESC)`
  ).run();
}

async function persistSuspiciousSyncs(
  env,
  accountId,
  monsterId,
  deviceId,
  findings,
  payload,
  detectedAt
) {
  for (const finding of findings) {
    await env.DB.prepare(
      `INSERT INTO suspicious_syncs (
          id, account_id, monster_id, device_id, reason, severity,
          requested_ranked_xp_delta, accepted_ranked_xp_delta,
          max_accepted_ranked_xp_delta, trusted_total_xp_after,
          payload_json, detected_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
    )
      .bind(
        crypto.randomUUID(),
        accountId,
        monsterId,
        deviceId,
        finding.reason,
        finding.severity,
        payload.ranked_progression.requestedDelta,
        payload.ranked_progression.acceptedDelta,
        payload.ranked_progression.maxAcceptedDelta,
        payload.ranked_progression.totalXp,
        JSON.stringify(payload),
        detectedAt
      )
      .run();
  }
}

export {
  computeAcceptedRankedProgression,
  determineVerificationState,
  extractBearerToken,
  evaluateSuspiciousSync,
  maxXpGainSince,
  normalizeSeverity,
  normalizeVerificationStatus,
  parseSuspiciousSyncQuery,
  progressionFromTotalXp,
  requireAdminToken,
  stageForLevel,
  totalXpForLevel,
  validateProfileSnapshot,
};
