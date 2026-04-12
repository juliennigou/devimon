CREATE TABLE IF NOT EXISTS accounts (
  account_id TEXT PRIMARY KEY,
  github_user_id INTEGER NOT NULL UNIQUE,
  username TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  session_token TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(account_id)
);

CREATE TABLE IF NOT EXISTS pending_device_logins (
  login_id TEXT PRIMARY KEY,
  device_code TEXT NOT NULL,
  user_code TEXT NOT NULL,
  verification_uri TEXT NOT NULL,
  interval_seconds INTEGER NOT NULL,
  expires_at TEXT NOT NULL,
  status TEXT NOT NULL,
  account_id TEXT,
  username TEXT,
  session_token TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS monsters (
  monster_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  level INTEGER NOT NULL CHECK (level >= 1),
  xp INTEGER NOT NULL CHECK (xp >= 0),
  total_xp INTEGER NOT NULL CHECK (total_xp >= 0),
  stage TEXT NOT NULL CHECK (stage IN ('Baby', 'Young', 'Evolved')),
  ranked_level INTEGER NOT NULL DEFAULT 1 CHECK (ranked_level >= 1),
  ranked_xp INTEGER NOT NULL DEFAULT 0 CHECK (ranked_xp >= 0),
  ranked_total_xp INTEGER NOT NULL DEFAULT 0 CHECK (ranked_total_xp >= 0),
  ranked_stage TEXT NOT NULL DEFAULT 'Baby' CHECK (ranked_stage IN ('Baby', 'Young', 'Evolved')),
  verification_status TEXT NOT NULL DEFAULT 'unverified' CHECK (verification_status IN ('verified', 'unverified')),
  verified_at TEXT,
  verification_reason TEXT,
  hunger REAL NOT NULL CHECK (hunger >= 0 AND hunger <= 100),
  energy REAL NOT NULL CHECK (energy >= 0 AND energy <= 100),
  mood REAL NOT NULL CHECK (mood >= 0 AND mood <= 100),
  last_active_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(account_id)
);

CREATE TABLE IF NOT EXISTS devices (
  device_id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(account_id)
);

CREATE TABLE IF NOT EXISTS sync_history (
  id TEXT PRIMARY KEY,
  monster_id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  received_at TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  FOREIGN KEY (monster_id) REFERENCES monsters(monster_id),
  FOREIGN KEY (device_id) REFERENCES devices(device_id)
);

CREATE TABLE IF NOT EXISTS suspicious_syncs (
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
  detected_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(account_id),
  FOREIGN KEY (monster_id) REFERENCES monsters(monster_id),
  FOREIGN KEY (device_id) REFERENCES devices(device_id)
);

CREATE INDEX IF NOT EXISTS idx_monsters_total_xp
  ON monsters (total_xp DESC, level DESC, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_sessions_account
  ON sessions (account_id);

CREATE INDEX IF NOT EXISTS idx_suspicious_syncs_account_detected
  ON suspicious_syncs (account_id, detected_at DESC);
