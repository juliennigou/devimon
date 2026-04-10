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
const terminalBody = document.querySelector("#onboarding-terminal");

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

    if (monsterCountEl) monsterCountEl.textContent = monsters.length;

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
      "https://api.github.com/repos/juliennigou/devimon",
      { headers: { Accept: "application/vnd.github.v3+json" } }
    );
    if (!res.ok) return;
    const data = await res.json();
    if (ghStarsEl && typeof data.stargazers_count === "number") {
      ghStarsEl.textContent = data.stargazers_count.toLocaleString();
    }
  } catch {
    // Stars are a nice-to-have
  }
}

// ── Smooth scroll for nav links ─────────────────────────────────────────
document.querySelectorAll('.nav-link[href^="#"]').forEach((link) => {
  link.addEventListener("click", () => {
    document.querySelectorAll(".nav-link").forEach((l) => l.classList.remove("active"));
    link.classList.add("active");
  });
});

// ── Onboarding terminal animation ──────────────────────────────────────
// Scripted sequence that loops: install → logs → launch → ASCII pet → restart
const ONBOARD_SCRIPT = [
  { type: "cmd",  text: "curl -fsSL https://get.devimon.dev | bash" },
  { type: "log",  text: "  Detecting platform... macOS ARM64", cls: "log-line" },
  { type: "log",  text: "  Fetching latest release... v0.1.2", cls: "log-line" },
  { type: "log",  text: "  Downloading devimon-macos-arm64...", cls: "log-line" },
  { type: "log",  text: "  Installing to /usr/local/bin/devimon", cls: "log-line log-ok" },
  { type: "log",  text: "  Devimon v0.1.2 installed.", cls: "log-line log-ok" },
  { type: "pause", ms: 600 },
  { type: "cmd",  text: "devimon spawn Kiara" },
  { type: "log",  text: "  Spawning new monster: Kiara", cls: "log-line" },
  { type: "log",  text: "  Species: Devimon  |  Stage: Baby  |  Level: 1", cls: "log-line log-ok" },
  { type: "pause", ms: 400 },
  { type: "cmd",  text: "devimon" },
  { type: "log",  text: "  Starting TUI...", cls: "log-line" },
  { type: "pause", ms: 300 },
  { type: "ascii", lines: [
    "  ┌──────────────────────────────────────┐",
    "  │         Kiara  ♥  Lv.1  Baby         │",
    "  │                                      │",
    "  │            ( o_o )                    │",
    "  │           (       )                   │",
    "  │            \\_____/                    │",
    "  │             /|||\\                     │",
    "  │            d     b                    │",
    "  │                                      │",
    "  │  Hunger ████████░░  80%              │",
    "  │  Energy ██████████  100%             │",
    "  │  Mood   ████████░░  80%              │",
    "  │                                      │",
    "  │  [F]eed  [P]lay  [R]est  [S]ync     │",
    "  └──────────────────────────────────────┘",
  ]},
  { type: "pause", ms: 2500 },
  { type: "cmd",  text: "devimon feed Kiara" },
  { type: "log",  text: "  You fed Kiara! Hunger → 100%  (+15 XP)", cls: "log-line log-ok" },
  { type: "pause", ms: 800 },
  { type: "cmd",  text: "devimon sync" },
  { type: "log",  text: "  Syncing Kiara to cloud...", cls: "log-line" },
  { type: "log",  text: "  ✓ Synced! Leaderboard rank: #42", cls: "log-line log-ok" },
  { type: "pause", ms: 1500 },
  { type: "clear" },
];

let onboardAbort = null;

function sleep(ms) {
  return new Promise((resolve) => {
    const id = setTimeout(resolve, ms);
    if (onboardAbort) onboardAbort.push(() => clearTimeout(id));
  });
}

function addLine(html, cls) {
  const div = document.createElement("div");
  div.className = "onboard-line" + (cls ? " " + cls : "");
  div.innerHTML = html;
  terminalBody.appendChild(div);
  terminalBody.scrollTop = terminalBody.scrollHeight;
  return div;
}

async function typeText(el, text, speed) {
  for (let i = 0; i < text.length; i++) {
    el.textContent += text[i];
    await sleep(speed);
  }
}

async function runOnboarding() {
  while (true) {
    for (const step of ONBOARD_SCRIPT) {
      switch (step.type) {
        case "cmd": {
          const line = addLine(
            '<span class="prompt">$</span> <span class="cmd"></span><span class="cursor-blink">█</span>',
          );
          const cmdSpan = line.querySelector(".cmd");
          const cursor = line.querySelector(".cursor-blink");
          await typeText(cmdSpan, step.text, 40);
          cursor.remove();
          await sleep(300);
          break;
        }
        case "log": {
          addLine(escapeHtml(step.text), step.cls || "log-line");
          await sleep(180);
          break;
        }
        case "ascii": {
          const pre = document.createElement("pre");
          pre.className = "onboard-line ascii-inline";
          pre.textContent = step.lines.join("\n");
          terminalBody.appendChild(pre);
          terminalBody.scrollTop = terminalBody.scrollHeight;
          await sleep(100);
          break;
        }
        case "pause": {
          await sleep(step.ms);
          break;
        }
        case "clear": {
          terminalBody.replaceChildren();
          await sleep(400);
          break;
        }
      }
    }
  }
}

// ── Boot ─────────────────────────────────────────────────────────────────
loadLeaderboard();
loadGitHubStars();
runOnboarding();
