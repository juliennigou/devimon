const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "authorization, content-type",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

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

async function readJson(request) {
  try {
    return await request.json();
  } catch {
    throw new HttpError(400, "request body must be valid JSON");
  }
}

async function requireSession(request, env) {
  const auth = request.headers.get("authorization");
  if (!auth || !auth.startsWith("Bearer ")) {
    throw new HttpError(401, "missing bearer token");
  }

  const token = auth.slice("Bearer ".length).trim();
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
  const body = await readJson(request);
  if (!body.device_id || !body.snapshot) {
    throw new HttpError(400, "device_id and snapshot are required");
  }

  const snapshot = validateSnapshot(body.snapshot);
  const syncedAt = nowIso();

  await env.DB.prepare(
    `INSERT INTO devices (device_id, account_id, last_seen_at, created_at)
      VALUES (?, ?, ?, ?)
      ON CONFLICT(device_id)
      DO UPDATE SET
        account_id = excluded.account_id,
        last_seen_at = excluded.last_seen_at`
  )
    .bind(body.device_id, session.account_id, syncedAt, syncedAt)
    .run();

  const existing = await env.DB.prepare(
    `SELECT monster_id
       FROM monsters
      WHERE account_id = ?`
  )
    .bind(session.account_id)
    .first();

  const monsterId = existing?.monster_id || body.monster_id || crypto.randomUUID();

  await env.DB.prepare(
    `INSERT INTO monsters (
        monster_id, account_id, name, level, xp, total_xp, stage,
        hunger, energy, mood, last_active_at, updated_at
      )
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(monster_id)
      DO UPDATE SET
        account_id = excluded.account_id,
        name = excluded.name,
        level = excluded.level,
        xp = excluded.xp,
        total_xp = excluded.total_xp,
        stage = excluded.stage,
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
      snapshot.level,
      snapshot.xp,
      snapshot.total_xp,
      snapshot.stage,
      snapshot.hunger,
      snapshot.energy,
      snapshot.mood,
      snapshot.last_active_at,
      syncedAt
    )
    .run();

  await env.DB.prepare(
    `INSERT INTO sync_history (id, monster_id, device_id, received_at, payload_json)
      VALUES (?, ?, ?, ?, ?)`
  )
    .bind(
      crypto.randomUUID(),
      monsterId,
      body.device_id,
      syncedAt,
      JSON.stringify({
        device_id: body.device_id,
        monster_id: body.monster_id || null,
        snapshot,
      })
    )
    .run();

  const rankRow = await env.DB.prepare(
    `SELECT COUNT(*) + 1 AS rank
       FROM monsters
      WHERE total_xp > ?
         OR (total_xp = ? AND level > ?)
         OR (total_xp = ? AND level = ? AND updated_at > ?)`
  )
    .bind(
      snapshot.total_xp,
      snapshot.total_xp,
      snapshot.level,
      snapshot.total_xp,
      snapshot.level,
      syncedAt
    )
    .first();

  return json({
    monster_id: monsterId,
    synced_at: syncedAt,
    leaderboard_rank: rankRow?.rank ? Number(rankRow.rank) : null,
  });
}

function validateSnapshot(snapshot) {
  const requiredStrings = ["name", "stage", "last_active_at"];
  for (const key of requiredStrings) {
    if (typeof snapshot[key] !== "string" || !snapshot[key].trim()) {
      throw new HttpError(400, `snapshot.${key} is required`);
    }
  }

  const numericKeys = ["level", "xp", "total_xp", "hunger", "energy", "mood"];
  for (const key of numericKeys) {
    if (typeof snapshot[key] !== "number" || Number.isNaN(snapshot[key])) {
      throw new HttpError(400, `snapshot.${key} must be a number`);
    }
  }

  return {
    name: snapshot.name.trim().slice(0, 40),
    level: Math.max(1, Math.floor(snapshot.level)),
    xp: Math.max(0, Math.floor(snapshot.xp)),
    total_xp: Math.max(0, Math.floor(snapshot.total_xp)),
    stage: snapshot.stage,
    hunger: clamp(snapshot.hunger, 0, 100),
    energy: clamp(snapshot.energy, 0, 100),
    mood: clamp(snapshot.mood, 0, 100),
    last_active_at: new Date(snapshot.last_active_at).toISOString(),
  };
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

async function handleLeaderboard(request, env) {
  const url = new URL(request.url);
  const requested = Number(url.searchParams.get("limit") || 20);
  const limit = Number.isFinite(requested)
    ? Math.min(Math.max(Math.floor(requested), 1), 100)
    : 20;

  const rows = await env.DB.prepare(
    `SELECT monster_id, name, level, total_xp, stage, updated_at, last_active_at
       FROM monsters
      ORDER BY total_xp DESC, level DESC, updated_at DESC
      LIMIT ?`
  )
    .bind(limit)
    .all();

  const monsters = (rows.results || []).map((row, index) => ({
    rank: index + 1,
    monster_id: row.monster_id,
    name: row.name,
    level: Number(row.level),
    total_xp: Number(row.total_xp),
    stage: row.stage,
    updated_at: row.updated_at,
    last_active_at: row.last_active_at,
  }));

  return json({
    generated_at: nowIso(),
    monsters,
  });
}
