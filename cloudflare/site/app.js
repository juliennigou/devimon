const DEFAULT_REMOTE_API_BASE = "https://devimon-api.julienigou33.workers.dev";

const API_BASE =
  window.DEVIMON_API_BASE_URL ||
  (["127.0.0.1", "localhost"].includes(window.location.hostname)
    ? "http://127.0.0.1:8787"
    : DEFAULT_REMOTE_API_BASE);

// ── DOM refs ────────────────────────────────────────────────────────────
const apiBaseDisplay = document.querySelector("#api-base-display");
const generatedAtEl = document.querySelector("#generated-at");
const refreshButton = document.querySelector("#refresh-button");
const statusBanner = document.querySelector("#status-banner");
const tbody = document.querySelector("#leaderboard-body");
const ghStarsEl = document.querySelector("#gh-stars");
const playerCountEl = document.querySelector("#player-count");
const monsterCountEl = document.querySelector("#monster-count");

if (apiBaseDisplay) apiBaseDisplay.textContent = `${API_BASE}/api`;

// ── Refresh ─────────────────────────────────────────────────────────────
refreshButton.addEventListener("click", () => loadLeaderboard());

// ── Status banner ───────────────────────────────────────────────────────
function setStatus(message, kind = "neutral") {
  statusBanner.textContent = message;
  statusBanner.className = "status-line";
  if (kind !== "neutral") statusBanner.classList.add(kind);
}

// ── Render leaderboard ──────────────────────────────────────────────────
function renderLeaderboard(monsters) {
  tbody.replaceChildren();

  if (!monsters.length) {
    const row = document.createElement("tr");
    row.className = "empty-row";
    row.innerHTML = `
      <td colspan="6">
        <span class="empty-ascii">
   ( ?_? )
  (       )
   \\_____/
    /|||\\
   d     b</span>
        ${escapeHtml(t("leaderboard.empty"))}
      </td>`;
    tbody.appendChild(row);
    return;
  }

  // Find max XP for bar scaling
  const maxXp = Math.max(...monsters.map((m) => m.total_xp || 0), 1);

  for (const monster of monsters) {
    const rank = rankDisplay(monster.rank);
    const sClass = stageClass(monster.stage);
    const xpPct = Math.round(((monster.total_xp || 0) / maxXp) * 100);
    const row = document.createElement("tr");
    row.style.animationDelay = `${monster.rank * 0.04}s`;
    row.innerHTML = `
      <td class="rank-cell ${rank.cls}">${rank.text}</td>
      <td>
        <div class="monster-cell">
          <span class="monster-name">${escapeHtml(monster.name)}</span>
          <span class="monster-id">${escapeHtml(monster.monster_id)}</span>
        </div>
      </td>
      <td><span class="stage-pill ${sClass}">${escapeHtml(monster.stage)}</span></td>
      <td>${monster.level}</td>
      <td>
        <div class="xp-cell">
          <span class="xp-number">${(monster.total_xp || 0).toLocaleString()}</span>
          <div class="xp-bar-track"><div class="xp-bar-fill" style="width:${xpPct}%"></div></div>
        </div>
      </td>
      <td><span class="time-ago">${timeAgo(monster.last_active_at)}</span></td>
    `;
    tbody.appendChild(row);
  }
}

// ── Load leaderboard ────────────────────────────────────────────────────
async function loadLeaderboard() {
  setStatus(t("leaderboard.loading"));
  refreshButton.disabled = true;
  try {
    const response = await fetch(`${API_BASE}/api/leaderboard?limit=25`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const data = await response.json();
    const monsters = data.monsters || [];
    renderLeaderboard(monsters);

    // Update stats
    if (monsterCountEl) monsterCountEl.textContent = monsters.length;

    // Estimate unique players (unique monster_id prefix or count)
    const playerSet = new Set(monsters.map((m) => m.monster_id?.split("-")[0]));
    if (playerCountEl)
      playerCountEl.textContent = playerSet.size || monsters.length;

    if (generatedAtEl && data.generated_at) {
      generatedAtEl.textContent = new Date(data.generated_at).toLocaleString();
    }

    setStatus(t("leaderboard.success"), "success");
  } catch (error) {
    setStatus(t("leaderboard.error") + error.message, "error");
  } finally {
    refreshButton.disabled = false;
  }
}

// ── GitHub stars ─────────────────────────────────────────────────────────
async function loadGitHubStars() {
  try {
    const res = await fetch(
      "https://api.github.com/repos/music-fam/devimon",
      { headers: { Accept: "application/vnd.github.v3+json" } }
    );
    if (!res.ok) return;
    const data = await res.json();
    if (ghStarsEl && typeof data.stargazers_count === "number") {
      ghStarsEl.textContent = data.stargazers_count.toLocaleString();
    }
  } catch {
    // Silently fail — stars are a nice-to-have
  }
}

// ── Smooth scroll for nav links ─────────────────────────────────────────
document.querySelectorAll('.nav-link[href^="#"]').forEach((link) => {
  link.addEventListener("click", (e) => {
    document.querySelectorAll(".nav-link").forEach((l) => l.classList.remove("active"));
    link.classList.add("active");
  });
});

// ── Boot ─────────────────────────────────────────────────────────────────
loadLeaderboard();
loadGitHubStars();
