const DEFAULT_REMOTE_API_BASE = "https://devimon-api.julienigou33.workers.dev";

const API_BASE =
  window.DEVIMON_API_BASE_URL ||
  (["127.0.0.1", "localhost"].includes(window.location.hostname)
    ? "http://127.0.0.1:8787"
    : DEFAULT_REMOTE_API_BASE);

const apiBaseEl = document.querySelector("#api-base");
const generatedAtEl = document.querySelector("#generated-at");
const refreshButton = document.querySelector("#refresh-button");
const statusBanner = document.querySelector("#status-banner");
const tbody = document.querySelector("#leaderboard-body");

apiBaseEl.textContent = `${API_BASE}/api`;

refreshButton.addEventListener("click", () => {
  loadLeaderboard();
});

function setStatus(message, kind = "neutral") {
  statusBanner.textContent = message;
  statusBanner.className = "status-banner";
  if (kind !== "neutral") {
    statusBanner.classList.add(kind);
  }
}

function timeAgo(value) {
  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) {
    return "Unknown";
  }

  const diffSeconds = Math.max(
    0,
    Math.floor((Date.now() - timestamp.getTime()) / 1000),
  );
  if (diffSeconds < 60) {
    return `${diffSeconds}s ago`;
  }
  if (diffSeconds < 3600) {
    return `${Math.floor(diffSeconds / 60)}m ago`;
  }
  if (diffSeconds < 86400) {
    return `${Math.floor(diffSeconds / 3600)}h ago`;
  }
  return `${Math.floor(diffSeconds / 86400)}d ago`;
}

function renderLeaderboard(monsters) {
  tbody.replaceChildren();

  if (!monsters.length) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 6;
    cell.textContent =
      "No synced monsters yet. Run `devimon login` and `devimon sync` from a terminal first.";
    row.appendChild(cell);
    tbody.appendChild(row);
    return;
  }

  for (const monster of monsters) {
    const row = document.createElement("tr");
    row.innerHTML = `
      <td>#${monster.rank}</td>
      <td>
        <div class="monster-cell">
          <span class="monster-name">${escapeHtml(monster.name)}</span>
          <span class="monster-id">${escapeHtml(monster.monster_id)}</span>
        </div>
      </td>
      <td><span class="stage-pill">${escapeHtml(monster.stage)}</span></td>
      <td>${monster.level}</td>
      <td>${monster.total_xp.toLocaleString()}</td>
      <td>${timeAgo(monster.last_active_at)}</td>
    `;
    tbody.appendChild(row);
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

async function loadLeaderboard() {
  setStatus("Loading leaderboard…");
  refreshButton.disabled = true;
  try {
    const response = await fetch(`${API_BASE}/api/leaderboard?limit=25`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const data = await response.json();
    renderLeaderboard(data.monsters || []);
    generatedAtEl.textContent = new Date(data.generated_at).toLocaleString();
    setStatus("Leaderboard up to date.", "success");
  } catch (error) {
    setStatus(`Failed to load leaderboard: ${error.message}`, "error");
  } finally {
    refreshButton.disabled = false;
  }
}

loadLeaderboard();
