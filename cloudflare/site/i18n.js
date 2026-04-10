const translations = {
  en: {
    "nav.leaderboard": "Leaderboard",
    "nav.about": "About",
    "hero.cmd": "cat welcome.txt",
    "hero.cmd2": "devimon stats --global",
    "stats.stars": "GitHub Stars",
    "stats.players": "Players",
    "stats.monsters": "Monsters",
    "stats.version": "Version",
    "leaderboard.title":
      "┌── LEADERBOARD ──────────────────────────────────────┐",
    "leaderboard.refresh": "[↻ Refresh]",
    "leaderboard.loading": "Loading leaderboard...",
    "leaderboard.success": "Leaderboard up to date.",
    "leaderboard.error": "Failed to load leaderboard: ",
    "leaderboard.empty":
      'No synced monsters yet. Run "devimon login" and "devimon sync" from a terminal.',
    "table.rank": "#",
    "table.monster": "Monster",
    "table.stage": "Stage",
    "table.level": "LVL",
    "table.xp": "XP",
    "table.active": "Last Active",
    "about.title": "┌── ABOUT ─────────────────────────────────────────────┐",
    "about.cmd": "cat README.md",
    "about.terminal.title": "Terminal Native",
    "about.terminal.desc":
      "Devimon lives in your terminal. Feed, play, and evolve your monster while you code.",
    "about.cloud.title": "Cloud Sync",
    "about.cloud.desc":
      "Sync your monsters to the cloud and compete on the global leaderboard.",
    "about.evolve.title": "3 Stages",
    "about.evolve.desc":
      "Baby → Young → Evolved. Each stage unlocks new ASCII art and abilities.",
    "footer.made": "Made with",
    "footer.and": "and",
    "footer.updated": "Last updated:",
  },
  fr: {
    "nav.leaderboard": "Classement",
    "nav.about": "À propos",
    "hero.cmd": "cat bienvenue.txt",
    "hero.cmd2": "devimon stats --global",
    "stats.stars": "Étoiles GitHub",
    "stats.players": "Joueurs",
    "stats.monsters": "Monstres",
    "stats.version": "Version",
    "leaderboard.title":
      "┌── CLASSEMENT ───────────────────────────────────────┐",
    "leaderboard.refresh": "[↻ Actualiser]",
    "leaderboard.loading": "Chargement du classement...",
    "leaderboard.success": "Classement à jour.",
    "leaderboard.error": "Échec du chargement : ",
    "leaderboard.empty":
      'Aucun monstre synchronisé. Lancez "devimon login" puis "devimon sync" depuis un terminal.',
    "table.rank": "#",
    "table.monster": "Monstre",
    "table.stage": "Stade",
    "table.level": "NIV",
    "table.xp": "XP",
    "table.active": "Dernière activité",
    "about.title":
      "┌── À PROPOS ──────────────────────────────────────────┐",
    "about.cmd": "cat LISEZMOI.md",
    "about.terminal.title": "Terminal Natif",
    "about.terminal.desc":
      "Devimon vit dans votre terminal. Nourrissez, jouez et faites évoluer votre monstre en codant.",
    "about.cloud.title": "Synchro Cloud",
    "about.cloud.desc":
      "Synchronisez vos monstres dans le cloud et rivalisez sur le classement mondial.",
    "about.evolve.title": "3 Stades",
    "about.evolve.desc":
      "Bébé → Jeune → Évolué. Chaque stade débloque de nouveaux arts ASCII et capacités.",
    "footer.made": "Fait avec",
    "footer.and": "et du",
    "footer.updated": "Dernière mise à jour :",
  },
};

function setLanguage(lang) {
  const dict = translations[lang];
  if (!dict) return;

  document.documentElement.setAttribute("data-lang", lang);
  localStorage.setItem("devimon-lang", lang);

  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n");
    if (dict[key] !== undefined) {
      el.textContent = dict[key];
    }
  });

  document.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === lang);
  });
}

function t(key) {
  const lang = document.documentElement.getAttribute("data-lang") || "en";
  return translations[lang]?.[key] || translations.en[key] || key;
}

function initI18n() {
  const saved = localStorage.getItem("devimon-lang");
  const browserLang = navigator.language?.startsWith("fr") ? "fr" : "en";
  const lang = saved || browserLang;

  setLanguage(lang);

  document.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.addEventListener("click", () => setLanguage(btn.dataset.lang));
  });
}

initI18n();
