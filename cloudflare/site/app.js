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
const installCommandEl = document.querySelector("#install-command");
const installHintEl = document.querySelector("#install-hint");
const installOsLabelEl = document.querySelector("#install-os-label");
const installCopyButton = document.querySelector("#install-copy-button");
const installModeButtons = document.querySelectorAll("[data-install-mode]");

if (apiBaseDisplay) apiBaseDisplay.textContent = `${API_BASE}/api`;

const INSTALL_VARIANTS = {
  unix: {
    osLabel: "macOS / Linux",
    prompt: "$",
    command:
      "curl -fsSL https://raw.githubusercontent.com/juliennigou/devimon/main/install.sh | bash",
    hintKey: "install.hint.unix",
    logs: [
      "  Detecting platform... macOS / Linux",
      "  Fetching latest release... v0.1.10",
      "  Downloading devimon for your platform...",
      "  Installing to /usr/local/bin/devimon",
      "  Devimon v0.1.10 installed.",
    ],
  },
  windows: {
    osLabel: "Windows PowerShell",
    prompt: "PS>",
    command:
      "irm https://raw.githubusercontent.com/juliennigou/devimon/main/install.ps1 | iex",
    hintKey: "install.hint.windows",
    logs: [
      "  Detecting platform... Windows",
      "  Fetching latest release... v0.1.10",
      "  Downloading devimon-windows-x86_64.exe...",
      "  Installing to %USERPROFILE%\\.devimon\\bin\\devimon.exe",
      "  Devimon v0.1.10 installed.",
    ],
  },
};

function detectInstallVariant() {
  const platform =
    navigator.userAgentData?.platform || navigator.platform || navigator.userAgent || "";
  const normalized = platform.toLowerCase();
  if (normalized.includes("win")) {
    return "windows";
  }
  return "unix";
}

let currentInstallMode = "auto";

function getInstallVariantConfig() {
  const variant =
    currentInstallMode === "auto" ? detectInstallVariant() : currentInstallMode;
  return INSTALL_VARIANTS[variant] || INSTALL_VARIANTS.unix;
}

function updateToggleSlider({ disableTransition = false } = {}) {
  const switchEl = document.querySelector(".install-mode-switch");
  if (!switchEl) return false;
  const activeBtn = switchEl.querySelector(".install-mode-btn.active");
  const slider = switchEl.querySelector(".toggle-slider");
  if (!activeBtn || !slider) return false;
  if (!activeBtn.offsetWidth) return false;
  if (disableTransition) {
    slider.classList.add("toggle-slider--no-transition");
  }
  slider.style.left = activeBtn.offsetLeft + "px";
  slider.style.width = activeBtn.offsetWidth + "px";
  if (disableTransition) {
    requestAnimationFrame(() => slider.classList.remove("toggle-slider--no-transition"));
  }
  return true;
}

function syncToggleSlider(options = {}) {
  requestAnimationFrame(() => {
    updateToggleSlider(options);
  });
}

function renderInstallPanel() {
  const config = getInstallVariantConfig();
  if (installCommandEl) installCommandEl.textContent = config.command;
  if (installOsLabelEl) installOsLabelEl.textContent = config.osLabel;
  if (installHintEl) {
    installHintEl.setAttribute("data-i18n", config.hintKey);
    installHintEl.textContent = t(config.hintKey);
  }
  installModeButtons.forEach((button) => {
    button.classList.toggle("active", button.dataset.installMode === currentInstallMode);
  });
}

function fallbackCopyText(text) {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "absolute";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  const ok = document.execCommand("copy");
  document.body.removeChild(textarea);
  if (!ok) throw new Error("copy failed");
}

async function copyInstallCommand() {
  const { command } = getInstallVariantConfig();
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(command);
  } else {
    fallbackCopyText(command);
  }
  const original = t("install.copy");
  installCopyButton.textContent = t("install.copy.done");
  window.setTimeout(() => {
    installCopyButton.textContent = original;
  }, 1200);
}

renderInstallPanel();

syncToggleSlider({ disableTransition: true });

const installModeSwitchEl = document.querySelector(".install-mode-switch");
const pageEl = document.querySelector("#page");

if (installModeSwitchEl && "ResizeObserver" in window) {
  const installModeResizeObserver = new ResizeObserver(() => {
    updateToggleSlider();
  });
  installModeResizeObserver.observe(installModeSwitchEl);
}

if (pageEl && "MutationObserver" in window) {
  const pageVisibilityObserver = new MutationObserver(() => {
    if (pageEl.classList.contains("hidden")) return;
    syncToggleSlider({ disableTransition: true });
    pageVisibilityObserver.disconnect();
  });
  pageVisibilityObserver.observe(pageEl, { attributes: true, attributeFilter: ["class"] });
}

if (document.fonts?.ready) {
  document.fonts.ready.then(() => {
    updateToggleSlider({ disableTransition: true });
  });
}

if (installCopyButton) {
  installCopyButton.addEventListener("click", () => {
    copyInstallCommand().catch(() => {});
  });
}

let onboardingRunId = 0;

function resetOnboardingSurface() {
  terminalBody.replaceChildren();
  document.querySelectorAll(".wandering-monster").forEach((el) => el.remove());
}

function restartOnboarding() {
  onboardingRunId += 1;
  resetOnboardingSurface();
  runOnboarding(onboardingRunId);
}

installModeButtons.forEach((button) => {
  button.addEventListener("click", () => {
    currentInstallMode = button.dataset.installMode || "auto";
    renderInstallPanel();
    updateToggleSlider();
    restartOnboarding();
  });
});

window.addEventListener("resize", () => updateToggleSlider());

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
    const art = stageAscii(monster.stage);
    const ghUser = monster.github_username
      ? `<a class="gh-user" href="https://github.com/${escapeHtml(monster.github_username)}" target="_blank" rel="noopener">@${escapeHtml(monster.github_username)}</a>`
      : "";
    const row = document.createElement("tr");
    row.style.animationDelay = `${monster.rank * 0.04}s`;
    row.innerHTML = `
      <td class="rank-cell ${rank.cls}">${rank.text}</td>
      <td class="art-cell"><span class="stage-art">${art}</span></td>
      <td>
        <div class="monster-cell">
          <span class="monster-name">${escapeHtml(monster.name)}</span>
          ${ghUser}
        </div>
      </td>
      <td><span class="stage-pill ${sClass}">${escapeHtml(monster.stage)}</span></td>
      <td class="td-center">${monster.level}</td>
      <td class="td-xp">
        <div class="xp-bar-bg" style="--xp-pct:${xpPct}%">
          <span class="xp-number">${(monster.total_xp || 0).toLocaleString()}</span>
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
      { headers: { Accept: "application/vnd.github.v3+json" } },
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
    document
      .querySelectorAll(".nav-link")
      .forEach((l) => l.classList.remove("active"));
    link.classList.add("active");
  });
});

// ── Onboarding terminal animation ──────────────────────────────────────
// Runs once: install → spawn → launch TUI → monster escapes into the page
function buildOnboardScript() {
  const config = getInstallVariantConfig();
  return [
    {
      type: "cmd",
      text: config.command,
    },
    ...config.logs.map((text, index) => ({
      type: "log",
      text,
      cls: index >= 3 ? "log-line log-ok" : "log-line",
    })),
    { type: "pause", ms: 500 },
    { type: "cmd", text: "devimon spawn Kiara" },
    { type: "log", text: "  Spawning new monster: Kiara", cls: "log-line" },
    {
      type: "log",
      text: "  Species: Devimon  |  Stage: Baby  |  Level: 1",
      cls: "log-line log-ok",
    },
    { type: "pause", ms: 400 },
    { type: "cmd", text: "devimon" },
    { type: "log", text: "  Starting TUI...", cls: "log-line" },
    { type: "pause", ms: 300 },
    {
      type: "ascii",
      lines: [
        "  ┌──────────────────────────────────────┐",
        "  │         Kiara  ♥  Lv.1  Baby         │",
        "  │                                      │",
        "  │           .-^-.                     │",
        "  │        .-( ^o^ )-.                  │",
        "  │          /|___|\\                     │",
        "  │          d_/ \\_b                     │",
        "  │                                      │",
        "  │  Hunger ████████░░  80%              │",
        "  │  Energy ██████████  100%             │",
        "  │  Mood   ████████░░  80%              │",
        "  │                                      │",
        "  │  [F]eed  [P]lay  [R]est  [S]ync     │",
        "  └──────────────────────────────────────┘",
      ],
    },
    { type: "pause", ms: 2000 },
    { type: "escape" },
  ];
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
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

async function runOnboarding(runId = onboardingRunId) {
  for (const step of buildOnboardScript()) {
    if (runId !== onboardingRunId) {
      return;
    }
    switch (step.type) {
      case "cmd": {
        const { prompt } = getInstallVariantConfig();
        const line = addLine(
          `<span class="prompt">${escapeHtml(prompt)}</span> <span class="cmd"></span><span class="cursor-blink">█</span>`,
        );
        const cmdSpan = line.querySelector(".cmd");
        const cursor = line.querySelector(".cursor-blink");
        await typeText(cmdSpan, step.text, 38);
        cursor.remove();
        await sleep(280);
        break;
      }
      case "log": {
        addLine(escapeHtml(step.text), step.cls || "log-line");
        await sleep(160);
        break;
      }
      case "ascii": {
        const pre = document.createElement("pre");
        pre.className = "onboard-line ascii-inline";
        pre.textContent = step.lines.join("\n");
        terminalBody.appendChild(pre);
        terminalBody.scrollTop = terminalBody.scrollHeight;
        await sleep(80);
        break;
      }
      case "pause": {
        await sleep(step.ms);
        break;
      }
      case "escape": {
        launchWanderingMonster();
        break;
      }
    }
  }
}

// ── Wandering monster ────────────────────────────────────────────────────
// Monster "escapes" from the terminal and wanders the page freely.

const MONSTER_FRAMES = [
  // frame A — legs apart
  "   .-^-.\n .-( ^o^ )-.\n  /|___|\\ \n  b_/ \\_d",
  // frame B — legs together
  "   .-^-.\n .-( ^o^ )-.\n  /|___|\\ \n  d_/ \\_b",
];

function launchWanderingMonster() {
  // Find where the terminal window is so we can start from there
  const terminalEl = document.querySelector(".terminal-window");
  const rect = terminalEl
    ? terminalEl.getBoundingClientRect()
    : { left: 100, bottom: 200 };

  const el = document.createElement("pre");
  el.className = "wandering-monster";
  el.textContent = MONSTER_FRAMES[0];
  document.body.appendChild(el);

  // Start at bottom of the terminal panel
  let x = rect.left + 40;
  let y = window.scrollY + rect.bottom - 60;

  el.style.left = x + "px";
  el.style.top = y + "px";

  // Animate: smooth Lissajous wander across the full page
  let tick = 0;
  let frame = 0;
  const SPEED = 0.6; // pixels per ms base

  // Target points shift over time using sine waves
  function getTarget(t) {
    const margin = 60;
    const maxX = window.innerWidth - margin - 120;
    const maxY = document.body.scrollHeight - margin - 80;
    return {
      tx: margin + (Math.sin(t * 0.00031) * 0.5 + 0.5) * maxX,
      ty: margin + (Math.cos(t * 0.00019) * 0.5 + 0.5) * maxY,
    };
  }

  let lastTime = null;

  function step(now) {
    if (!lastTime) lastTime = now;
    const dt = now - lastTime;
    lastTime = now;
    tick += dt;

    const { tx, ty } = getTarget(tick);
    const dx = tx - x;
    const dy = ty - y;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist > 1) {
      const move = Math.min(SPEED * dt, dist);
      x += (dx / dist) * move;
      y += (dy / dist) * move;
    }

    el.style.left = Math.round(x) + "px";
    el.style.top = Math.round(y) + "px";

    // Flip horizontally based on direction
    el.style.transform = dx < 0 ? "scaleX(-1)" : "scaleX(1)";

    // Alternate walk frame every ~300ms
    if (Math.floor(tick / 300) % 2 !== frame) {
      frame = Math.floor(tick / 300) % 2;
      el.textContent = MONSTER_FRAMES[frame];
    }

    requestAnimationFrame(step);
  }

  requestAnimationFrame(step);
}

// ── Boot ─────────────────────────────────────────────────────────────────
loadLeaderboard();
loadGitHubStars();
restartOnboarding();
