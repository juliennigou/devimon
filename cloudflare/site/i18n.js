const translations = {
  en: {
    "nav.home": "Home",
    "nav.leaderboard": "Leaderboard",
    "nav.about": "About",
    "stats.players": "Players",
    "stats.monsters": "Monsters",
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
    "install.label": "Install",
    "install.copy": "[Copy]",
    "install.copy.done": "[Copied]",
    "install.mode.auto": "Auto",
    "install.mode.unix": "Unix",
    "install.mode.windows": "Windows",
    "install.hint.unix": "Uses the shell installer for Unix environments.",
    "install.hint.windows":
      "Use this in PowerShell for a native Windows install.",
    "install.hint.unknown":
      "Defaulting to the Unix installer. Switch to the PowerShell command on Windows.",
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
    "nav.home": "Accueil",
    "nav.leaderboard": "Classement",
    "nav.about": "À propos",
    "stats.players": "Joueurs",
    "stats.monsters": "Monstres",
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
    "install.label": "Installation",
    "install.copy": "[Copier]",
    "install.copy.done": "[Copié]",
    "install.mode.auto": "Auto",
    "install.mode.unix": "Unix",
    "install.mode.windows": "Windows",
    "install.hint.unix":
      "Utilise l'installateur shell pour les environnements Unix.",
    "install.hint.windows":
      "À lancer dans PowerShell pour une installation Windows native.",
    "install.hint.unknown":
      "Installateur Unix affiché par défaut. Utilisez la commande PowerShell sous Windows.",
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
  const lang = saved || "en";

  setLanguage(lang);

  document.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.addEventListener("click", () => setLanguage(btn.dataset.lang));
  });
}

initI18n();
