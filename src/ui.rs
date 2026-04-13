use crate::actions;
use crate::cloud::{self, PollLoginStatus};
use crate::dino::{self, DinoCommand, DinoGamePhase, DinoGameSession};
use crate::display::{self, MoodState};
use crate::monster::{Monster, Species};
use crate::save::{self, SaveFile};
use crate::watcher;
use crate::xp;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap, block::Title},
};
use std::io::{self, Stdout};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const GAME_TICK_RATE: Duration = Duration::from_millis(500);
const ANIMATION_FRAME_RATE: Duration = Duration::from_millis(60);
const FLASH_DURATION: Duration = Duration::from_secs(3);
const SYNC_RATE: Duration = Duration::from_secs(20);
const DINO_MAX_STEPS_PER_LOOP: u8 = 5;
const STARTER_SPECIES: [Species; 3] = [Species::Ember, Species::Tide, Species::Bloom];

// ── Menu ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuTab {
    Home,
    Collection,
    Games,
    Team,
    Settings,
}

impl MenuTab {
    fn label(self) -> &'static str {
        match self {
            MenuTab::Home => "Home",
            MenuTab::Collection => "Collection",
            MenuTab::Games => "Games",
            MenuTab::Team => "Team",
            MenuTab::Settings => "Settings",
        }
    }
}

const MENU_ITEMS: &[MenuTab] = &[
    MenuTab::Home,
    MenuTab::Collection,
    MenuTab::Games,
    MenuTab::Team,
    MenuTab::Settings,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsAction {
    SyncNow,
    Disconnect,
}

impl SettingsAction {
    fn label(self) -> &'static str {
        match self {
            SettingsAction::SyncNow => "Sync now",
            SettingsAction::Disconnect => "Disconnect GitHub",
        }
    }

    fn description(self) -> &'static str {
        match self {
            SettingsAction::SyncNow => {
                "Push cloud progression and verification status immediately."
            }
            SettingsAction::Disconnect => "Clear the local cloud session on this device.",
        }
    }
}

const SETTINGS_ACTIONS: &[SettingsAction] = &[SettingsAction::SyncNow, SettingsAction::Disconnect];

#[derive(Clone, Copy, PartialEq, Eq)]
enum MiniGame {
    DinoRun,
}

impl MiniGame {
    fn label(self) -> &'static str {
        match self {
            MiniGame::DinoRun => "Dino Run",
        }
    }

    fn description(self) -> &'static str {
        match self {
            MiniGame::DinoRun => "Jump over cacti with your main monster.",
        }
    }
}

const MINI_GAMES: &[MiniGame] = &[MiniGame::DinoRun];

enum ActiveMiniGame {
    Dino(DinoGameSession),
}

#[derive(Clone, Copy)]
enum StartupChoiceMode {
    FirstLaunch,
    Returning,
}

// ── App state ─────────────────────────────────────────────────────────────────

enum AppState {
    StartupChoice {
        state: SaveFile,
        mode: StartupChoiceMode,
        cursor: usize,
        animation_tick: u64,
    },
    OnboardingIntro {
        animation_tick: u64,
    },
    OnboardingEggSelect {
        cursor: usize,
        animation_tick: u64,
    },
    OnboardingName {
        species: Species,
        name_input: String,
        animation_tick: u64,
    },
    OnboardingConfirm {
        species: Species,
        name_input: String,
        confirm_choice: usize,
        animation_tick: u64,
    },
    LoginFlow {
        state: SaveFile,
        login: cloud::StartLoginResponse,
        result_rx: mpsc::Receiver<Result<cloud::AccountEnvelope, String>>,
        animation_tick: u64,
    },
    Running {
        state: SaveFile,
        flash: Option<Flash>,
        last_sync_attempt: Instant,
        selected_tab: MenuTab,
        collection_cursor: usize,
        games_cursor: usize,
        settings_cursor: usize,
        active_game: Option<ActiveMiniGame>,
        /// true = ↑↓ navigate content panel; false = ↑↓ navigate sidebar
        content_focused: bool,
        settings_logout_confirm: bool,
        settings_logout_choice: usize,
        animation_tick: u64,
    },
    Quit,
}

struct Flash {
    message: String,
    kind: FlashKind,
    created_at: Instant,
}

#[derive(Clone, Copy)]
enum FlashKind {
    Success,
    Error,
    Info,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    if let Ok(cwd) = std::env::current_dir() {
        thread::spawn(move || {
            let _ = watcher::watch_silent(&cwd);
        });
    }

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn check_for_update_bg(result: Arc<Mutex<Option<String>>>) {
    thread::spawn(move || {
        let current = env!("CARGO_PKG_VERSION");
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let resp = client
            .get("https://api.github.com/repos/juliennigou/devimon/releases/latest")
            .header("User-Agent", "devimon-updater")
            .header("Accept", "application/vnd.github+json")
            .send();
        let tag = resp
            .ok()
            .filter(|r| r.status().is_success())
            .and_then(|r| r.json::<serde_json::Value>().ok())
            .and_then(|j| {
                j["tag_name"]
                    .as_str()
                    .map(|s| s.trim_start_matches('v').to_string())
            });
        if let Some(latest) = tag {
            if latest != current {
                if let Ok(mut guard) = result.lock() {
                    *guard = Some(latest);
                }
            }
        }
    });
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let update_available: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    check_for_update_bg(Arc::clone(&update_available));

    let mut app = initial_state()?;
    let mut last_game_tick = Instant::now();
    let mut last_animation_frame = Instant::now();
    let mut last_dino_step = Instant::now();
    let mut dino_accumulator = Duration::ZERO;

    loop {
        let update_ver = update_available.lock().ok().and_then(|g| g.clone());
        terminal.draw(|f| draw(f, &app, update_ver.as_deref()))?;

        let mut timeout = std::cmp::min(
            GAME_TICK_RATE.saturating_sub(last_game_tick.elapsed()),
            ANIMATION_FRAME_RATE.saturating_sub(last_animation_frame.elapsed()),
        );
        if has_active_dino(&app) {
            timeout = std::cmp::min(timeout, dino::SIM_STEP.saturating_sub(dino_accumulator));
        }
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if matches!(
                    key.kind,
                    KeyEventKind::Press | KeyEventKind::Repeat | KeyEventKind::Release
                ) {
                    handle_key(&mut app, key)?;
                }
            }
        }

        let now = Instant::now();
        if has_active_dino(&app) {
            dino_accumulator = dino_accumulator.saturating_add(now - last_dino_step);
            last_dino_step = now;
            let mut steps = 0;
            while dino_accumulator >= dino::SIM_STEP && steps < DINO_MAX_STEPS_PER_LOOP {
                step_dino(&mut app);
                dino_accumulator = dino_accumulator.saturating_sub(dino::SIM_STEP);
                steps += 1;
            }
            if steps == DINO_MAX_STEPS_PER_LOOP {
                dino_accumulator = Duration::ZERO;
            }
        } else {
            last_dino_step = now;
            dino_accumulator = Duration::ZERO;
        }

        if last_animation_frame.elapsed() >= ANIMATION_FRAME_RATE {
            animate(&mut app);
            last_animation_frame = now;
        }

        if last_game_tick.elapsed() >= GAME_TICK_RATE {
            tick(&mut app)?;
            last_game_tick = now;
        }

        if matches!(app, AppState::Quit) {
            break;
        }
    }
    Ok(())
}

fn initial_state() -> io::Result<AppState> {
    match save::load_state()? {
        None => Ok(AppState::OnboardingIntro { animation_tick: 0 }),
        Some(state) => {
            if state.cloud.account.is_none() {
                Ok(AppState::StartupChoice {
                    state,
                    mode: StartupChoiceMode::Returning,
                    cursor: 0,
                    animation_tick: 0,
                })
            } else {
                Ok(make_running_state(
                    state,
                    Some(Flash {
                        message: "Bon retour ! Ton monstre t'attendait.".to_string(),
                        kind: FlashKind::Info,
                        created_at: Instant::now(),
                    }),
                ))
            }
        }
    }
}

fn make_running_state(state: SaveFile, flash: Option<Flash>) -> AppState {
    AppState::Running {
        state,
        flash,
        last_sync_attempt: Instant::now() - SYNC_RATE,
        selected_tab: MenuTab::Home,
        collection_cursor: 0,
        games_cursor: 0,
        settings_cursor: 0,
        active_game: None,
        content_focused: false,
        settings_logout_confirm: false,
        settings_logout_choice: 0,
        animation_tick: 0,
    }
}

fn starter_species(cursor: usize) -> Species {
    STARTER_SPECIES
        .get(cursor)
        .copied()
        .unwrap_or(Species::Ember)
}

fn starter_species_name(species: Species) -> &'static str {
    species.label()
}

fn starter_species_description(species: Species) -> &'static str {
    match species {
        Species::Ember => "Fierce flame cub. Burns bright through long debugging marathons.",
        Species::Tide => "Tidepool drifter. Calm, focused, and relentless under pressure.",
        Species::Bloom => "Rooted sapling. Patient grower with deep, steady resolve.",
    }
}

fn starter_default_name(species: Species) -> &'static str {
    match species {
        Species::Ember => "Embit",
        Species::Tide => "Driplet",
        Species::Bloom => "Sprout",
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Element {
    Fire,
    Water,
    Grass,
}

fn starter_element(species: Species) -> Element {
    match species {
        Species::Ember => Element::Fire,
        Species::Tide => Element::Water,
        Species::Bloom => Element::Grass,
    }
}

fn element_label(element: Element) -> &'static str {
    match element {
        Element::Fire => "FIRE",
        Element::Water => "WATER",
        Element::Grass => "GRASS",
    }
}

fn element_accent(element: Element) -> Color {
    match element {
        Element::Fire => Color::LightRed,
        Element::Water => Color::LightCyan,
        Element::Grass => Color::LightGreen,
    }
}

fn element_deep(element: Element) -> Color {
    match element {
        Element::Fire => Color::Rgb(20, 0, 0),
        Element::Water => Color::Rgb(0, 15, 45),
        Element::Grass => Color::Rgb(0, 18, 5),
    }
}

fn element_tagline(element: Element) -> &'static str {
    match element {
        Element::Fire => "Burning will. Bursts of intensity.",
        Element::Water => "Flowing focus. Calm under load.",
        Element::Grass => "Steady growth. Roots that hold.",
    }
}

fn hatch_starter(species: Species, name_input: &str) -> SaveFile {
    let name = if name_input.trim().is_empty() {
        starter_default_name(species).to_string()
    } else {
        name_input.trim().to_string()
    };
    SaveFile::new(Monster::spawn_with_species(name, species))
}

fn starter_preview_monster(species: Species, name_input: &str) -> Monster {
    let name = if name_input.trim().is_empty() {
        starter_default_name(species).to_string()
    } else {
        name_input.trim().to_string()
    };
    Monster::spawn_with_species(name, species)
}

fn starter_egg_art(species: Species, selected: bool, tick: u64) -> Vec<String> {
    let wobble_left = selected && (tick / 6).is_multiple_of(2);
    let wobble_pad = if wobble_left { " " } else { "" };
    let motif = match species {
        Species::Ember => {
            if selected && (tick / 4).is_multiple_of(2) {
                "**"
            } else {
                "^^"
            }
        }
        Species::Tide => {
            if selected && (tick / 4).is_multiple_of(2) {
                "~~"
            } else {
                "≈≈"
            }
        }
        Species::Bloom => {
            if selected && (tick / 4).is_multiple_of(2) {
                "\\/"
            } else {
                "//"
            }
        }
    };

    vec![
        format!("{wobble_pad}    .-^^-.    "),
        format!("{wobble_pad}  .'  {motif}  '.  "),
        format!("{wobble_pad} /  .----.  \\ "),
        format!("{wobble_pad}|  (______)  |"),
        format!("{wobble_pad} \\   ____   / "),
        format!("{wobble_pad}  '--------'  "),
    ]
}

fn onboarding_title_art() -> &'static [&'static str] {
    &[
        "██████╗  ███████╗ ██╗   ██╗ ██╗ ███╗   ███╗  ██████╗  ███╗   ██╗",
        "██╔══██╗ ██╔════╝ ██║   ██║ ██║ ████╗ ████║ ██╔═══██╗ ████╗  ██║",
        "██║  ██║ █████╗   ██║   ██║ ██║ ██╔████╔██║ ██║   ██║ ██╔██╗ ██║",
        "██║  ██║ ██╔══╝   ╚██╗ ██╔╝ ██║ ██║╚██╔╝██║ ██║   ██║ ██║╚██╗██║",
        "██████╔╝ ███████╗  ╚████╔╝  ██║ ██║ ╚═╝ ██║ ╚██████╔╝ ██║ ╚████║",
        "╚═════╝  ╚══════╝   ╚═══╝   ╚═╝ ╚═╝     ╚═╝  ╚═════╝  ╚═╝  ╚═══╝",
    ]
}

// ── Tick ──────────────────────────────────────────────────────────────────────

fn tick(app: &mut AppState) -> io::Result<()> {
    let login_result = if let AppState::LoginFlow { result_rx, .. } = app {
        match result_rx.try_recv() {
            Ok(result) => Some(result),
            Err(_) => None,
        }
    } else {
        None
    };

    if let Some(result) = login_result {
        if let AppState::LoginFlow { state, .. } = app {
            match result {
                Ok(account) => {
                    let username = account.username.clone();
                    state.cloud.account = Some(account.into());
                    save::mark_dirty(state);
                    cloud::sync_state(state).ok();
                    save::save_state(state).ok();
                    *app = make_running_state(
                        state.clone(),
                        Some(Flash {
                            message: format!("Logged in as @{}!", username),
                            kind: FlashKind::Success,
                            created_at: Instant::now(),
                        }),
                    );
                }
                Err(e) => {
                    *app = make_running_state(
                        state.clone(),
                        Some(Flash {
                            message: format!("Login failed: {}", e),
                            kind: FlashKind::Error,
                            created_at: Instant::now(),
                        }),
                    );
                }
            }
        }
        return Ok(());
    }

    if let AppState::Running {
        state,
        flash,
        last_sync_attempt,
        active_game,
        ..
    } = app
    {
        if active_game.is_some() {
            return Ok(());
        }

        let idx = state.active_monster_idx();
        let (decayed, xp_gained) =
            xp::tick_monster_progress(&mut state.monsters[idx]).unwrap_or((false, 0));
        if decayed {
            save::mark_dirty(state);
        }
        if xp_gained > 0 {
            save::record_ranked_xp_delta(state, xp_gained);
        }
        if let Some(new_stage) = state.monsters[idx].check_evolution() {
            let name = state.monsters[idx].name.clone();
            save::mark_dirty(state);
            *flash = Some(Flash {
                message: format!("✨ {} a évolué — {} !", name, new_stage.label()),
                kind: FlashKind::Success,
                created_at: Instant::now(),
            });
        } else if xp_gained > 0 {
            *flash = Some(Flash {
                message: format!("+{} XP", xp_gained),
                kind: FlashKind::Info,
                created_at: Instant::now(),
            });
        }

        maybe_sync(state, flash, last_sync_attempt, false);
        save::save_state(state).ok();
    }
    Ok(())
}

fn animate(app: &mut AppState) {
    match app {
        AppState::OnboardingIntro { animation_tick }
        | AppState::OnboardingEggSelect { animation_tick, .. }
        | AppState::OnboardingName { animation_tick, .. }
        | AppState::OnboardingConfirm { animation_tick, .. }
        | AppState::StartupChoice { animation_tick, .. }
        | AppState::LoginFlow { animation_tick, .. }
        | AppState::Running { animation_tick, .. } => {
            *animation_tick = animation_tick.wrapping_add(1);
        }
        _ => {}
    }
}

fn has_active_dino(app: &AppState) -> bool {
    matches!(
        app,
        AppState::Running {
            active_game: Some(ActiveMiniGame::Dino(_)),
            ..
        }
    )
}

fn step_dino(app: &mut AppState) {
    if let AppState::Running {
        state,
        flash,
        active_game,
        ..
    } = app
    {
        if let Some(ActiveMiniGame::Dino(session)) = active_game {
            session.update();
            if session.phase == DinoGamePhase::Running {
                let monster = state.active_monster();
                if dino::has_collision(monster, session) {
                    finish_dino_run(state, flash, session);
                }
            }
        }
    }
}

// ── Input ─────────────────────────────────────────────────────────────────────

fn start_login_flow(app: &mut AppState, state: SaveFile) {
    match cloud::start_login() {
        Ok(login) => {
            let (tx, rx) = mpsc::channel();
            spawn_login_poller(login.login_id.clone(), login.interval_seconds, tx);
            *app = AppState::LoginFlow {
                state,
                login,
                result_rx: rx,
                animation_tick: 0,
            };
        }
        Err(e) => {
            *app = make_running_state(
                state,
                Some(Flash {
                    message: format!("Impossible de démarrer la connexion: {}", e),
                    kind: FlashKind::Error,
                    created_at: Instant::now(),
                }),
            );
        }
    }
}

fn persist_and_quit(app: &mut AppState) {
    let to_save: Option<SaveFile> = match app {
        AppState::Running { state, .. } => Some(state.clone()),
        AppState::LoginFlow { state, .. } => Some(state.clone()),
        AppState::StartupChoice { state, .. } => Some(state.clone()),
        _ => None,
    };
    if let Some(ref state) = to_save {
        let _ = save::save_state(state);
    }
    *app = AppState::Quit;
}

fn handle_key(app: &mut AppState, key: KeyEvent) -> io::Result<()> {
    let code = key.code;
    let mods = key.modifiers;

    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
        persist_and_quit(app);
        return Ok(());
    }

    match app {
        AppState::StartupChoice { state, cursor, .. } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                *cursor = 1 - *cursor;
            }
            KeyCode::Char('l') => {
                let state_owned = state.clone();
                start_login_flow(app, state_owned);
            }
            KeyCode::Char('n') => {
                *app = make_running_state(state.clone(), None);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if *cursor == 0 {
                    let state_owned = state.clone();
                    start_login_flow(app, state_owned);
                } else {
                    *app = make_running_state(state.clone(), None);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => persist_and_quit(app),
            _ => {}
        },

        AppState::LoginFlow { state, .. } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Esc | KeyCode::Char('q') => {
                *app = make_running_state(
                    state.clone(),
                    Some(Flash {
                        message: "Connexion annulée.".to_string(),
                        kind: FlashKind::Info,
                        created_at: Instant::now(),
                    }),
                );
            }
            _ => {}
        },

        AppState::OnboardingIntro { .. } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Enter | KeyCode::Char(' ') => {
                *app = AppState::OnboardingEggSelect {
                    cursor: 0,
                    animation_tick: 0,
                };
            }
            KeyCode::Esc | KeyCode::Char('q') => *app = AppState::Quit,
            _ => {}
        },

        AppState::OnboardingEggSelect {
            cursor,
            animation_tick,
        } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Left | KeyCode::Up => {
                if *cursor == 0 {
                    *cursor = STARTER_SPECIES.len() - 1;
                } else {
                    *cursor -= 1;
                }
            }
            KeyCode::Right | KeyCode::Down => {
                *cursor = (*cursor + 1) % STARTER_SPECIES.len();
            }
            KeyCode::Enter => {
                *app = AppState::OnboardingName {
                    species: starter_species(*cursor),
                    name_input: String::new(),
                    animation_tick: *animation_tick,
                };
            }
            KeyCode::Esc => {
                *app = AppState::OnboardingIntro {
                    animation_tick: *animation_tick,
                };
            }
            _ => {}
        },

        AppState::OnboardingName {
            species,
            name_input,
            animation_tick,
        } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Char(c) if name_input.chars().count() < 20 && !c.is_control() => {
                name_input.push(c)
            }
            KeyCode::Backspace => {
                name_input.pop();
            }
            KeyCode::Enter => {
                *app = AppState::OnboardingConfirm {
                    species: *species,
                    name_input: name_input.clone(),
                    confirm_choice: 0,
                    animation_tick: *animation_tick,
                };
            }
            KeyCode::Esc => {
                let species = *species;
                *app = AppState::OnboardingEggSelect {
                    cursor: STARTER_SPECIES
                        .iter()
                        .position(|starter| *starter == species)
                        .unwrap_or(0),
                    animation_tick: *animation_tick,
                };
            }
            _ => {}
        },

        AppState::OnboardingConfirm {
            species,
            name_input,
            confirm_choice,
            animation_tick,
        } => match code {
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                *confirm_choice = 1 - *confirm_choice;
            }
            KeyCode::Enter => {
                if *confirm_choice == 0 {
                    let state = hatch_starter(*species, name_input);
                    save::save_state(&state).ok();
                    *app = AppState::StartupChoice {
                        state,
                        mode: StartupChoiceMode::FirstLaunch,
                        cursor: 0,
                        animation_tick: 0,
                    };
                } else {
                    *app = AppState::OnboardingName {
                        species: *species,
                        name_input: name_input.clone(),
                        animation_tick: *animation_tick,
                    };
                }
            }
            KeyCode::Esc => {
                *app = AppState::OnboardingName {
                    species: *species,
                    name_input: name_input.clone(),
                    animation_tick: *animation_tick,
                };
            }
            _ => {}
        },

        AppState::Running {
            state,
            flash,
            last_sync_attempt,
            selected_tab,
            collection_cursor,
            games_cursor,
            settings_cursor,
            active_game,
            content_focused,
            settings_logout_confirm,
            settings_logout_choice,
            ..
        } => match code {
            KeyCode::Char('q') | KeyCode::Esc
                if key.kind != KeyEventKind::Release
                    && matches!(active_game, Some(ActiveMiniGame::Dino(_))) =>
            {
                *active_game = None;
                save::save_state(state).ok();
            }
            KeyCode::Char(' ') | KeyCode::Up | KeyCode::Down | KeyCode::Enter
                if matches!(active_game, Some(ActiveMiniGame::Dino(_))) =>
            {
                if let Some(ActiveMiniGame::Dino(session)) = active_game {
                    match (code, key.kind) {
                        (
                            KeyCode::Char(' ') | KeyCode::Up,
                            KeyEventKind::Press | KeyEventKind::Repeat,
                        ) => {
                            session.handle_command(DinoCommand::JumpPressed);
                        }
                        (KeyCode::Char(' ') | KeyCode::Up, KeyEventKind::Release) => {
                            session.handle_command(DinoCommand::JumpReleased);
                        }
                        (KeyCode::Down, KeyEventKind::Press | KeyEventKind::Repeat) => {
                            session.handle_command(DinoCommand::DuckPressed);
                        }
                        (KeyCode::Down, KeyEventKind::Release) => {
                            session.handle_command(DinoCommand::DuckReleased);
                        }
                        (KeyCode::Enter, KeyEventKind::Press | KeyEventKind::Repeat) => {
                            session.handle_command(DinoCommand::Restart);
                        }
                        _ => {}
                    }
                }
            }
            KeyCode::Char('p')
                if key.kind != KeyEventKind::Release
                    && matches!(active_game, Some(ActiveMiniGame::Dino(_))) =>
            {
                if let Some(ActiveMiniGame::Dino(session)) = active_game {
                    session.handle_command(DinoCommand::TogglePause);
                }
            }
            _ if key.kind == KeyEventKind::Release => {}
            KeyCode::Char('q') | KeyCode::Esc => {
                if *settings_logout_confirm {
                    *settings_logout_confirm = false;
                    *settings_logout_choice = 0;
                } else if *content_focused {
                    *content_focused = false;
                } else {
                    persist_and_quit(app)
                }
            }

            // ── Navigation ────────────────────────────────────────────────
            // ↑↓ in sidebar mode → switch tabs
            KeyCode::Up if !*content_focused => {
                let idx = MENU_ITEMS
                    .iter()
                    .position(|&t| t == *selected_tab)
                    .unwrap_or(0);
                if idx > 0 {
                    *selected_tab = MENU_ITEMS[idx - 1];
                    *collection_cursor = 0;
                    *games_cursor = 0;
                    *settings_cursor = 0;
                    *settings_logout_confirm = false;
                    *settings_logout_choice = 0;
                }
            }
            KeyCode::Down if !*content_focused => {
                let idx = MENU_ITEMS
                    .iter()
                    .position(|&t| t == *selected_tab)
                    .unwrap_or(0);
                if idx + 1 < MENU_ITEMS.len() {
                    *selected_tab = MENU_ITEMS[idx + 1];
                    *collection_cursor = 0;
                    *games_cursor = 0;
                    *settings_cursor = 0;
                    *settings_logout_confirm = false;
                    *settings_logout_choice = 0;
                }
            }
            // → enters content panel (only on tabs that have interactive content)
            KeyCode::Right
                if !*content_focused
                    && matches!(
                        *selected_tab,
                        MenuTab::Collection | MenuTab::Games | MenuTab::Settings
                    ) =>
            {
                *content_focused = true;
            }
            // ← exits content panel back to sidebar
            KeyCode::Left if *content_focused && active_game.is_none() => {
                if *settings_logout_confirm {
                    *settings_logout_confirm = false;
                    *settings_logout_choice = 0;
                } else {
                    *content_focused = false;
                }
            }
            // ↑↓ in content mode → navigate collection items
            KeyCode::Up if *content_focused && *selected_tab == MenuTab::Collection => {
                if *collection_cursor > 0 {
                    *collection_cursor -= 1;
                }
            }
            KeyCode::Down if *content_focused && *selected_tab == MenuTab::Collection => {
                if *collection_cursor + 1 < state.monsters.len() {
                    *collection_cursor += 1;
                }
            }
            KeyCode::Up
                if *content_focused && *selected_tab == MenuTab::Games && active_game.is_none() =>
            {
                if *games_cursor > 0 {
                    *games_cursor -= 1;
                }
            }
            KeyCode::Down
                if *content_focused && *selected_tab == MenuTab::Games && active_game.is_none() =>
            {
                if *games_cursor + 1 < MINI_GAMES.len() {
                    *games_cursor += 1;
                }
            }
            KeyCode::Up if *content_focused && *selected_tab == MenuTab::Settings => {
                if *settings_logout_confirm {
                    if *settings_logout_choice > 0 {
                        *settings_logout_choice -= 1;
                    }
                } else if *settings_cursor > 0 {
                    *settings_cursor -= 1;
                }
            }
            KeyCode::Down if *content_focused && *selected_tab == MenuTab::Settings => {
                if *settings_logout_confirm {
                    if *settings_logout_choice < 1 {
                        *settings_logout_choice += 1;
                    }
                } else if *settings_cursor + 1 < SETTINGS_ACTIONS.len() {
                    *settings_cursor += 1;
                }
            }
            KeyCode::Enter if *content_focused && *selected_tab == MenuTab::Collection => {
                if let Some(monster) = state.monsters.get(*collection_cursor) {
                    let id = monster.id.clone();
                    let name = monster.name.clone();
                    if id != state.active_monster_id {
                        state.set_active(&id);
                        save::mark_dirty(state);
                        save::save_state(state).ok();
                        *flash = Some(Flash {
                            message: format!("{} is now your main monster!", name),
                            kind: FlashKind::Success,
                            created_at: Instant::now(),
                        });
                    } else {
                        *flash = Some(Flash {
                            message: format!("{} is already your main monster.", name),
                            kind: FlashKind::Info,
                            created_at: Instant::now(),
                        });
                    }
                }
            }
            KeyCode::Enter
                if *content_focused && *selected_tab == MenuTab::Games && active_game.is_none() =>
            {
                if active_game.is_none() {
                    let seed = state.active_monster().total_xp as u64
                        + state.games.dino.best_time_ms
                        + *games_cursor as u64
                        + 1;
                    *active_game = match MINI_GAMES.get(*games_cursor).copied() {
                        Some(MiniGame::DinoRun) => {
                            Some(ActiveMiniGame::Dino(DinoGameSession::new(seed)))
                        }
                        None => None,
                    };
                }
            }
            KeyCode::Enter if *content_focused && *selected_tab == MenuTab::Settings => {
                if *settings_logout_confirm {
                    if *settings_logout_choice == 0 {
                        *settings_logout_confirm = false;
                    } else {
                        let username = state
                            .cloud
                            .account
                            .as_ref()
                            .map(|account| account.username.clone());
                        save::clear_session(state);
                        save::save_state(state).ok();
                        *last_sync_attempt = Instant::now() - SYNC_RATE;
                        *settings_logout_confirm = false;
                        *settings_logout_choice = 0;
                        *flash = Some(Flash {
                            message: match username {
                                Some(username) => format!("Disconnected @{}.", username),
                                None => "No active GitHub session.".to_string(),
                            },
                            kind: FlashKind::Info,
                            created_at: Instant::now(),
                        });
                    }
                } else if state.cloud.account.is_some() {
                    match SETTINGS_ACTIONS.get(*settings_cursor) {
                        Some(SettingsAction::SyncNow) => match cloud::sync_state(state) {
                            Ok(sync) => {
                                save::save_state(state).ok();
                                *last_sync_attempt = Instant::now();
                                *flash = Some(Flash {
                                    message: sync_flash_message(&sync),
                                    kind: FlashKind::Info,
                                    created_at: Instant::now(),
                                });
                            }
                            Err(err) => {
                                *flash = Some(Flash {
                                    message: format!("cloud sync failed: {}", err),
                                    kind: FlashKind::Error,
                                    created_at: Instant::now(),
                                });
                            }
                        },
                        Some(SettingsAction::Disconnect) => {
                            *settings_logout_confirm = true;
                            *settings_logout_choice = 0;
                        }
                        None => {}
                    }
                }
            }

            // ── Home actions ──────────────────────────────────────────────
            KeyCode::Char('f') if *selected_tab == MenuTab::Home => {
                let result = actions::feed(state.active_monster_mut());
                if result.is_ok() {
                    save::mark_dirty(state);
                }
                *flash = Some(make_flash(result));
                maybe_sync(state, flash, last_sync_attempt, true);
                save::save_state(state).ok();
            }
            KeyCode::Char('p') if *selected_tab == MenuTab::Home => {
                let result = actions::play(state.active_monster_mut());
                if result.is_ok() {
                    save::mark_dirty(state);
                }
                *flash = Some(make_flash(result));
                maybe_sync(state, flash, last_sync_attempt, true);
                save::save_state(state).ok();
            }
            KeyCode::Char('r') if *selected_tab == MenuTab::Home => {
                let result = actions::rest(state.active_monster_mut());
                if result.is_ok() {
                    save::mark_dirty(state);
                }
                *flash = Some(make_flash(result));
                maybe_sync(state, flash, last_sync_attempt, true);
                save::save_state(state).ok();
            }
            _ => {}
        },

        AppState::Quit => {}
    }
    Ok(())
}

fn make_flash(result: Result<String, String>) -> Flash {
    match result {
        Ok(msg) => Flash {
            message: msg,
            kind: FlashKind::Success,
            created_at: Instant::now(),
        },
        Err(msg) => Flash {
            message: msg,
            kind: FlashKind::Error,
            created_at: Instant::now(),
        },
    }
}

fn maybe_sync(
    state: &mut SaveFile,
    flash: &mut Option<Flash>,
    last_sync_attempt: &mut Instant,
    force: bool,
) {
    if state.cloud.account.is_none() || !state.cloud.sync_dirty {
        return;
    }
    if !force && last_sync_attempt.elapsed() < SYNC_RATE {
        return;
    }
    *last_sync_attempt = Instant::now();
    match cloud::sync_state(state) {
        Ok(sync) => {
            save::save_state(state).ok();
            if should_replace_flash(flash) {
                *flash = Some(Flash {
                    message: sync_flash_message(&sync),
                    kind: FlashKind::Info,
                    created_at: Instant::now(),
                });
            }
        }
        Err(err) => {
            if should_replace_flash(flash) {
                *flash = Some(Flash {
                    message: format!("cloud sync failed: {}", err),
                    kind: FlashKind::Error,
                    created_at: Instant::now(),
                });
            }
        }
    }
}

fn sync_flash_message(sync: &cloud::SyncResponse) -> String {
    let mut parts = vec!["☁️ Sync ok".to_string()];

    if let Some(rank) = sync.leaderboard_rank {
        parts.push(format!("rank #{}", rank));
    }
    if let Some(level) = sync.cloud_level {
        parts.push(format!("cloud lv.{}", level));
    }
    if let Some(accepted) = sync.accepted_xp_delta {
        parts.push(format!("+{} XP", accepted));
    }

    let message = parts.join(" · ");

    match (sync.requested_xp_delta, sync.accepted_xp_delta) {
        (Some(requested), Some(accepted)) if requested > accepted => {
            format!("{} · capped from +{} XP", message, requested)
        }
        _ => message,
    }
}

fn should_replace_flash(flash: &Option<Flash>) -> bool {
    match flash {
        None => true,
        Some(f) => f.created_at.elapsed() >= FLASH_DURATION,
    }
}

fn spawn_login_poller(
    login_id: String,
    interval_seconds: u64,
    tx: mpsc::Sender<Result<cloud::AccountEnvelope, String>>,
) {
    thread::spawn(move || {
        let mut interval = Duration::from_secs(interval_seconds.max(1));
        loop {
            thread::sleep(interval);
            match cloud::poll_login(&login_id) {
                Ok(resp) => match resp.status {
                    PollLoginStatus::Pending => {
                        if let Some(next) = resp.interval_seconds {
                            interval = Duration::from_secs(next.max(1));
                        }
                    }
                    PollLoginStatus::Complete => {
                        match resp.account {
                            Some(account) => {
                                let _ = tx.send(Ok(account));
                            }
                            None => {
                                let _ = tx.send(Err("login completed without account data".into()));
                            }
                        }
                        return;
                    }
                    PollLoginStatus::Expired | PollLoginStatus::Denied => {
                        let _ = tx.send(Err(resp
                            .message
                            .unwrap_or_else(|| "login was not approved".into())));
                        return;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            }
        }
    });
}

// ── Top-level draw ────────────────────────────────────────────────────────────

fn draw(f: &mut ratatui::Frame, app: &AppState, update_available: Option<&str>) {
    let (status_label, status_color) = match app {
        AppState::Running { state, .. } if state.cloud.account.is_some() => {
            (" ● Online ", Color::Green)
        }
        AppState::Running { .. } => (" ● Offline ", Color::DarkGray),
        AppState::Quit => (" ● Offline ", Color::DarkGray),
        _ => (" ● Setup ", Color::Cyan),
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Title::from(Span::styled(
            status_label,
            Style::default().fg(status_color),
        )))
        .title(
            Title::from(Span::styled(
                " Devimon 🐾 ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
        )
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = outer.inner(f.area());
    f.render_widget(outer, f.area());

    match app {
        AppState::StartupChoice {
            state,
            mode,
            cursor,
            animation_tick,
        } => draw_startup_choice(f, inner, state, *mode, *cursor, *animation_tick),
        AppState::OnboardingIntro { animation_tick } => {
            draw_onboarding_intro(f, inner, *animation_tick)
        }
        AppState::OnboardingEggSelect {
            cursor,
            animation_tick,
        } => draw_onboarding_egg_select(f, inner, *cursor, *animation_tick),
        AppState::OnboardingName {
            species,
            name_input,
            animation_tick,
        } => draw_onboarding_name(f, inner, *species, name_input, *animation_tick),
        AppState::OnboardingConfirm {
            species,
            name_input,
            confirm_choice,
            animation_tick,
        } => draw_onboarding_confirm(
            f,
            inner,
            *species,
            name_input,
            *confirm_choice,
            *animation_tick,
        ),
        AppState::LoginFlow {
            login,
            animation_tick,
            ..
        } => draw_login_flow(f, inner, login, *animation_tick),
        AppState::Running {
            state,
            flash,
            selected_tab,
            collection_cursor,
            games_cursor,
            settings_cursor,
            active_game,
            content_focused,
            settings_logout_confirm,
            settings_logout_choice,
            animation_tick,
            ..
        } => draw_running(
            f,
            inner,
            state,
            flash,
            *selected_tab,
            *collection_cursor,
            *games_cursor,
            *settings_cursor,
            active_game.as_ref(),
            *content_focused,
            *settings_logout_confirm,
            *settings_logout_choice,
            *animation_tick,
            update_available,
        ),
        AppState::Quit => {}
    }
}

// ── Running layout ────────────────────────────────────────────────────────────

fn draw_running(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    flash: &Option<Flash>,
    selected_tab: MenuTab,
    collection_cursor: usize,
    games_cursor: usize,
    settings_cursor: usize,
    active_game: Option<&ActiveMiniGame>,
    content_focused: bool,
    settings_logout_confirm: bool,
    settings_logout_choice: usize,
    animation_tick: u64,
    update_available: Option<&str>,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)])
        .split(area);

    draw_sidebar(f, cols[0], selected_tab, state, content_focused);
    draw_content(
        f,
        cols[1],
        state,
        flash,
        selected_tab,
        collection_cursor,
        games_cursor,
        settings_cursor,
        active_game,
        content_focused,
        settings_logout_confirm,
        settings_logout_choice,
        animation_tick,
        update_available,
    );
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn draw_sidebar(
    f: &mut ratatui::Frame,
    area: Rect,
    selected: MenuTab,
    state: &SaveFile,
    content_focused: bool,
) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // monster mini-header
            Constraint::Length(1), // divider
            Constraint::Min(0),    // menu items
            Constraint::Length(1), // nav hint
        ])
        .split(inner);

    // Monster mini-header (active monster)
    let active = state.active_monster();
    let header = vec![
        Line::from(Span::styled(
            active.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("lv.{}", active.level),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(active.stage.label(), Style::default().fg(Color::Blue)),
        ]),
    ];
    f.render_widget(Paragraph::new(header), rows[0]);

    // Divider
    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(rows[1].width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        rows[1],
    );

    // Menu items — dim the highlight when focus is in the content panel
    let arrow_color = if content_focused {
        Color::DarkGray
    } else {
        Color::Magenta
    };
    let selected_text_color = if content_focused {
        Color::DarkGray
    } else {
        Color::White
    };
    let mut lines: Vec<Line> = vec![Line::from("")];
    for &tab in MENU_ITEMS {
        if tab == selected {
            lines.push(Line::from(vec![
                Span::styled(
                    " ▶  ",
                    Style::default()
                        .fg(arrow_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    tab.label(),
                    Style::default()
                        .fg(selected_text_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(tab.label(), Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(lines), rows[2]);

    // Nav hint
    let hint = if content_focused {
        " ← back to menu"
    } else {
        " ↑↓ navigate"
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        rows[3],
    );
}

// ── Content dispatcher ────────────────────────────────────────────────────────

fn draw_content(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    flash: &Option<Flash>,
    selected_tab: MenuTab,
    collection_cursor: usize,
    games_cursor: usize,
    settings_cursor: usize,
    active_game: Option<&ActiveMiniGame>,
    content_focused: bool,
    settings_logout_confirm: bool,
    settings_logout_choice: usize,
    animation_tick: u64,
    update_available: Option<&str>,
) {
    match selected_tab {
        MenuTab::Home => draw_home(f, area, state, flash, animation_tick / 2, update_available),
        MenuTab::Collection => {
            draw_collection(f, area, state, collection_cursor, flash, content_focused)
        }
        MenuTab::Games => draw_games(f, area, state, games_cursor, active_game, content_focused),
        MenuTab::Settings => draw_settings(
            f,
            area,
            state,
            settings_cursor,
            content_focused,
            settings_logout_confirm,
            settings_logout_choice,
        ),
        tab => draw_coming_soon(f, area, tab),
    }
}

// ── Home ──────────────────────────────────────────────────────────────────────

fn draw_home(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    flash: &Option<Flash>,
    animation_tick: u64,
    update_available: Option<&str>,
) {
    let has_update = update_available.is_some();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(if has_update {
            vec![
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
        } else {
            vec![
                Constraint::Min(0),
                Constraint::Length(0),
                Constraint::Length(2),
            ]
        })
        .split(area);

    draw_monster_panel(f, rows[0], state.active_monster(), flash, animation_tick);
    draw_stats_panel(f, rows[0], state.active_monster());

    if let Some(latest) = update_available {
        let current = env!("CARGO_PKG_VERSION");
        let pulse = if (animation_tick / 8) % 2 == 0 {
            "↑"
        } else {
            "⬆"
        };
        let banner = Line::from(vec![
            Span::styled(format!(" {} ", pulse), Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("Update available: v{} → v{}", current, latest),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  —  run ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "devimon update",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to upgrade", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(banner).alignment(Alignment::Center), rows[1]);
    }

    draw_footer(f, rows[2], state);
}

fn draw_monster_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    monster: &Monster,
    flash: &Option<Flash>,
    animation_tick: u64,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // [0] header
            Constraint::Length(1), // [1] space
            Constraint::Min(5),    // [2] art (expandable for dragon flight)
            Constraint::Length(1), // [3] space
            Constraint::Length(1), // [4] xp gauge
            Constraint::Length(1), // [5] space
            Constraint::Length(1), // [6] personality
            Constraint::Length(1), // [7] flash
        ])
        .split(area);

    let header = Line::from(vec![
        Span::styled(
            monster.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("lv.{}", monster.level),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            monster.stage.label(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Paragraph::new(header).alignment(Alignment::Center),
        chunks[0],
    );

    let art_area = chunks[2];
    let scene = display::tui_scene(monster, animation_tick, art_area.width, art_area.height, 24);
    let sprite_h = scene.lines.len() as u16;
    let sprite_rect = Rect {
        x: art_area.x + scene.x.min(art_area.width.saturating_sub(1)),
        y: art_area.y + scene.y.min(art_area.height.saturating_sub(sprite_h)),
        width: art_area.width.saturating_sub(scene.x),
        height: sprite_h.min(art_area.height.saturating_sub(scene.y)),
    };

    let art: Vec<Line> = scene
        .lines
        .into_iter()
        .map(|l| {
            Line::from(Span::styled(
                l,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(art), sprite_rect);

    render_xp_gauge(f, center_rect(chunks[4], 55), monster);

    let personality = display::personality_text(monster);
    let p_color = match display::classify_mood(monster) {
        MoodState::Tired => Color::DarkGray,
        MoodState::Hungry => Color::Yellow,
        MoodState::Sad => Color::Red,
        MoodState::Proud => Color::Green,
        MoodState::Fine => Color::Cyan,
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            personality,
            Style::default().fg(p_color).add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center),
        chunks[6],
    );

    if let Some(flash) = flash {
        if flash.created_at.elapsed() < FLASH_DURATION {
            let color = match flash.kind {
                FlashKind::Success => Color::Green,
                FlashKind::Error => Color::Red,
                FlashKind::Info => Color::Cyan,
            };
            f.render_widget(
                Paragraph::new(Span::styled(
                    flash.message.clone(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center),
                chunks[7],
            );
        }
    }
}

fn draw_stats_panel(f: &mut ratatui::Frame, area: Rect, monster: &Monster) {
    const PANEL_W: u16 = 22;
    const PANEL_H: u16 = 5;
    if area.width < PANEL_W || area.height < PANEL_H {
        return;
    }
    let rect = Rect {
        x: area.x + area.width - PANEL_W,
        y: area.y,
        width: PANEL_W,
        height: PANEL_H,
    };
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stats ")
        .title_style(Style::default().fg(Color::DarkGray))
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
    f.render_widget(
        Paragraph::new(mini_bar_line("Faim   ", monster.hunger)),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(mini_bar_line("Énergie", monster.energy)),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(mini_bar_line("Moral  ", monster.mood)),
        rows[2],
    );
}

fn draw_footer(f: &mut ratatui::Frame, area: Rect, state: &SaveFile) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let keys = Line::from(vec![
        Span::styled(" f ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" feed   "),
        Span::styled(" p ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" play   "),
        Span::styled(" r ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" rest   "),
        Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::Red)),
        Span::raw(" quit"),
    ]);
    f.render_widget(Paragraph::new(keys).alignment(Alignment::Center), rows[0]);

    let cloud_line = if let Some(account) = &state.cloud.account {
        let suffix = if state.cloud.sync_dirty {
            "sync pending"
        } else {
            "cloud synced"
        };
        let verification = state
            .cloud
            .verification_status
            .map(|status| format!(" · {}", status.label()))
            .unwrap_or_default();
        let trusted = match (state.cloud.leaderboard_rank, state.cloud.cloud_level) {
            (Some(rank), Some(level)) => {
                format!(" · rank #{} · cloud lv.{}{}", rank, level, verification)
            }
            (Some(rank), None) => format!(" · official rank #{}{}", rank, verification),
            (None, Some(level)) => format!(" · cloud lv.{}{}", level, verification),
            (None, None) if !verification.is_empty() => verification,
            (None, None) => String::new(),
        };
        let pending = if state.cloud.pending_ranked_xp_delta > 0 {
            format!(" · pending +{}", state.cloud.pending_ranked_xp_delta)
        } else {
            String::new()
        };
        Line::from(vec![
            Span::styled("☁ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("@{} · {}{}{}", account.username, suffix, trusted, pending),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            "Offline — run `devimon login` to join the leaderboard",
            Style::default().fg(Color::DarkGray),
        ))
    };
    f.render_widget(
        Paragraph::new(cloud_line).alignment(Alignment::Center),
        rows[1],
    );
}

// ── Collection ────────────────────────────────────────────────────────────────

fn draw_collection(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    cursor: usize,
    flash: &Option<Flash>,
    content_focused: bool,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // spacer / divider
            Constraint::Min(0),    // cards
            Constraint::Length(1), // footer hint
        ])
        .split(area);

    // Title
    let n = state.monsters.len();
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Collection",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  —  {} monster{}", n, if n == 1 { "" } else { "s" }),
                Style::default().fg(Color::DarkGray),
            ),
        ])),
        rows[0],
    );

    // Divider
    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(rows[1].width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        rows[1],
    );

    // Cards
    const CARD_H: u16 = 4; // border top + 2 content lines + border bottom
    const CARD_GAP: u16 = 1;
    let cards_area = rows[2];

    for (i, monster) in state.monsters.iter().enumerate() {
        let y_offset = i as u16 * (CARD_H + CARD_GAP);
        if y_offset >= cards_area.height {
            break;
        }
        let h = CARD_H.min(cards_area.height - y_offset);
        let rect = Rect {
            x: cards_area.x,
            y: cards_area.y + y_offset,
            width: cards_area.width,
            height: h,
        };
        draw_monster_card(
            f,
            rect,
            monster,
            content_focused && i == cursor,
            monster.id == state.active_monster_id,
        );
    }

    // Flash or footer hint
    let hint_area = rows[3];
    if let Some(flash) = flash {
        if flash.created_at.elapsed() < FLASH_DURATION {
            let color = match flash.kind {
                FlashKind::Success => Color::Green,
                FlashKind::Error => Color::Red,
                FlashKind::Info => Color::Cyan,
            };
            f.render_widget(
                Paragraph::new(Span::styled(
                    flash.message.clone(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                hint_area,
            );
            return;
        }
    }
    let hint = if !content_focused {
        " → enter collection  ·  ↑↓ menu"
    } else if n > 1 {
        " ↑↓ select  ·  Enter set main  ·  ← back"
    } else {
        " spawn more with `devimon`  ·  ← back"
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        hint_area,
    );
}

fn draw_monster_card(
    f: &mut ratatui::Frame,
    area: Rect,
    monster: &Monster,
    selected: bool,
    is_main: bool,
) {
    if area.height < 4 {
        return;
    }

    let border_color = if selected {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    // ── Star badge + name in top-left title
    let star = if is_main {
        Span::styled(" ★ ", Style::default().fg(Color::Yellow))
    } else {
        Span::styled("   ", Style::default().fg(Color::DarkGray))
    };
    let name_style = if selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    // ── Level + stage right-aligned title
    let level_span = Span::styled(
        format!(" lv.{}", monster.level),
        Style::default().fg(if selected {
            Color::Yellow
        } else {
            Color::DarkGray
        }),
    );
    let stage_span = Span::styled(
        format!("  {}  ", monster.stage.label()),
        Style::default().fg(if selected {
            Color::Blue
        } else {
            Color::DarkGray
        }),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Title::from(Line::from(vec![
            star,
            Span::styled(monster.name.clone(), name_style),
            Span::raw(" "),
        ])))
        .title(Title::from(Line::from(vec![level_span, stage_span])).alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // ── Row 1: XP bar
    let xp_ratio = monster.xp as f64 / monster.xp_to_next() as f64;
    const XP_W: usize = 14;
    let filled = (xp_ratio * XP_W as f64).round() as usize;
    let empty = XP_W - filled;

    let mut xp_spans = vec![
        Span::styled(" XP ", Style::default().fg(Color::DarkGray)),
        Span::styled("█".repeat(filled), Style::default().fg(Color::Yellow)),
        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  {}/{}", monster.xp, monster.xp_to_next()),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    if is_main {
        xp_spans.push(Span::styled(
            "   MAIN",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(xp_spans)), inner_rows[0]);

    // ── Row 2: Needs mini bars (H / E / M)
    if inner.height >= 2 {
        let needs = Line::from(vec![
            Span::styled(" H ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                bar_chars(monster.hunger, 5),
                Style::default().fg(need_color(monster.hunger)),
            ),
            Span::styled("   E ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                bar_chars(monster.energy, 5),
                Style::default().fg(need_color(monster.energy)),
            ),
            Span::styled("   M ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                bar_chars(monster.mood, 5),
                Style::default().fg(need_color(monster.mood)),
            ),
        ]);
        f.render_widget(Paragraph::new(needs), inner_rows[1]);
    }
}

// ── Games ─────────────────────────────────────────────────────────────────────

fn draw_games(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    cursor: usize,
    active_game: Option<&ActiveMiniGame>,
    content_focused: bool,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Games",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  —  mini arcade", Style::default().fg(Color::DarkGray)),
        ])),
        rows[0],
    );

    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(rows[1].width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        rows[1],
    );

    match active_game {
        Some(ActiveMiniGame::Dino(session)) => draw_dino_game(f, rows[2], state, session),
        None => draw_games_menu(f, rows[2], state, cursor, content_focused),
    }

    let hint = match active_game {
        Some(ActiveMiniGame::Dino(session)) if session.phase == DinoGamePhase::Running => {
            " Space/↑ jump  ·  ↓ duck/drop  ·  Enter pause  ·  q exit game"
        }
        Some(ActiveMiniGame::Dino(session)) if session.phase == DinoGamePhase::Paused => {
            " Enter resume  ·  q back to games"
        }
        Some(ActiveMiniGame::Dino(_)) => " Space start/restart  ·  ↓ duck  ·  q back to games",
        None if !content_focused => " → enter games  ·  ↑↓ menu",
        None => " ↑↓ select  ·  Enter start  ·  ← back",
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        rows[3],
    );
}

fn draw_games_menu(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    cursor: usize,
    content_focused: bool,
) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Choose a mini game",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    for (index, game) in MINI_GAMES.iter().copied().enumerate() {
        let selected = content_focused && index == cursor;
        let arrow = if selected { "▶" } else { " " };
        let style = if selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", arrow), Style::default().fg(Color::Magenta)),
            Span::styled(game.label(), style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("    {}", game.description()),
            Style::default().fg(Color::DarkGray),
        )));
        if matches!(game, MiniGame::DinoRun) {
            lines.push(Line::from(Span::styled(
                format!(
                    "    Record: {}  ·  Unlocks queued: {}",
                    dino::format_duration_ms(state.games.dino.best_time_ms),
                    state.games.dino.pending_unlock_triggers
                ),
                Style::default().fg(Color::Yellow),
            )));
        }
        lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn draw_dino_game(f: &mut ratatui::Frame, area: Rect, state: &SaveFile, session: &DinoGameSession) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Dino Run ")
        .border_style(Style::default().fg(match session.phase {
            DinoGamePhase::Running => Color::Green,
            DinoGamePhase::Paused => Color::Yellow,
            DinoGamePhase::Ready | DinoGamePhase::Starting => Color::Cyan,
            DinoGamePhase::Crashed => Color::Red,
            DinoGamePhase::Exiting => Color::DarkGray,
        }));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(inner);

    let header = Line::from(vec![
        Span::styled(
            format!("Runner {}", state.active_monster().name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Time {}", dino::format_duration_ms(session.elapsed_ms)),
            Style::default().fg(Color::White),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Score {}", session.score),
            Style::default().fg(Color::LightYellow),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(
                "Record {}",
                dino::format_duration_ms(state.games.dino.best_time_ms)
            ),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Unlocks {}", state.games.dino.pending_unlock_triggers),
            Style::default().fg(Color::Green),
        ),
        Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Speed {:.1}", session.current_speed),
            Style::default().fg(Color::Magenta),
        ),
    ]);
    let status = dino::status_text(state.games.dino.best_time_ms, session);
    let header_lines = vec![
        header,
        Line::from(Span::styled(status, Style::default().fg(Color::DarkGray))),
    ];
    f.render_widget(
        Paragraph::new(header_lines).alignment(Alignment::Center),
        rows[0],
    );

    let world = dino::build_world(
        state.active_monster(),
        session,
        rows[1].width as usize,
        rows[1].height as usize,
    );
    let world_lines: Vec<Line> = world
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Magenta))))
        .collect();
    f.render_widget(
        Paragraph::new(world_lines).alignment(Alignment::Center),
        rows[1],
    );
}

fn draw_settings(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    cursor: usize,
    content_focused: bool,
    logout_confirm: bool,
    logout_choice: usize,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Settings",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  —  profile & device",
                Style::default().fg(Color::DarkGray),
            ),
        ])),
        rows[0],
    );

    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(rows[1].width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        rows[1],
    );

    let account = state.cloud.account.as_ref();
    let profile_lines = if let Some(account) = account {
        vec![
            Line::from(vec![
                Span::styled("GitHub", Style::default().fg(Color::DarkGray)),
                Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("@{}", account.username),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!("Profile  https://github.com/{}", account.username),
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                format!("Account ID  {}", account.account_id),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format!("Device ID   {}", state.cloud.device_id),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                if state.cloud.sync_dirty {
                    "Cloud state  sync pending"
                } else {
                    "Cloud state  synced"
                },
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                match (
                    state.cloud.leaderboard_rank,
                    state.cloud.cloud_level,
                    state.cloud.cloud_total_xp,
                    state.cloud.cloud_stage,
                ) {
                    (Some(rank), Some(level), Some(total_xp), Some(stage)) => {
                        format!(
                            "Rank #{}  ·  lv.{} {}  ·  {} XP",
                            rank,
                            level,
                            stage.label(),
                            total_xp,
                        )
                    }
                    (_, Some(level), Some(total_xp), Some(stage)) => format!(
                        "Cloud progression  lv.{} {}  ·  {} XP",
                        level,
                        stage.label(),
                        total_xp,
                    ),
                    _ => "Cloud progression  waiting for first sync".to_string(),
                },
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                match (
                    state.cloud.last_requested_xp_delta,
                    state.cloud.last_accepted_xp_delta,
                ) {
                    (Some(requested), Some(accepted)) if requested > accepted => format!(
                        "Last sync cap  requested +{} XP  ·  accepted +{} XP",
                        requested, accepted
                    ),
                    (Some(_), Some(accepted)) => {
                        format!(
                            "Last sync accept  +{} trusted XP  ·  pending +{}",
                            accepted, state.cloud.pending_ranked_xp_delta
                        )
                    }
                    _ => format!(
                        "Last sync accept  waiting for first sync  ·  pending +{}",
                        state.cloud.pending_ranked_xp_delta
                    ),
                },
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "GitHub  ·  disconnected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "Profile  connect from the startup login flow",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format!("Device ID   {}", state.cloud.device_id),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(""),
            Line::from(""),
        ]
    };
    f.render_widget(Paragraph::new(profile_lines), rows[2]);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Actions",
            Style::default().fg(Color::DarkGray),
        )),
        rows[3],
    );

    let mut action_lines = Vec::new();
    for (index, action) in SETTINGS_ACTIONS.iter().copied().enumerate() {
        let selected = content_focused && !logout_confirm && index == cursor;
        let enabled = account.is_some();
        let arrow = if selected { "▶" } else { " " };
        let label_style = if enabled {
            if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            }
        } else {
            Style::default().fg(Color::DarkGray)
        };
        action_lines.push(Line::from(vec![
            Span::styled(format!(" {} ", arrow), Style::default().fg(Color::Magenta)),
            Span::styled(action.label(), label_style),
        ]));
        action_lines.push(Line::from(Span::styled(
            format!("    {}", action.description()),
            Style::default().fg(Color::DarkGray),
        )));
        if !enabled {
            action_lines.push(Line::from(Span::styled(
                "    No active GitHub session.",
                Style::default().fg(Color::DarkGray),
            )));
        }
        action_lines.push(Line::from(""));
    }
    f.render_widget(Paragraph::new(action_lines), rows[4]);

    let hint = if logout_confirm {
        " ↑↓ choose  ·  Enter confirm  ·  Esc cancel"
    } else if !content_focused {
        " → enter settings  ·  ↑↓ menu"
    } else {
        " ↑↓ select  ·  Enter run  ·  ← back"
    };
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray))),
        rows[5],
    );

    if logout_confirm {
        draw_logout_confirm_modal(f, area, account.map(|a| a.username.as_str()), logout_choice);
    }
}

fn draw_logout_confirm_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    username: Option<&str>,
    choice: usize,
) {
    let rect = center_rect_with_size(area, 44, 8);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm Disconnect ")
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let prompt = match username {
        Some(username) => format!("Disconnect @{} on this device?", username),
        None => "Disconnect the current GitHub session?".to_string(),
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                prompt,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "This only clears the local session.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );

    let cancel_style = if choice == 0 {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let disconnect_style = if choice == 1 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "▶ ",
                Style::default().fg(if choice == 0 {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled("Cancel", cancel_style),
        ]))
        .alignment(Alignment::Center),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "▶ ",
                Style::default().fg(if choice == 1 {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled("Disconnect", disconnect_style),
        ]))
        .alignment(Alignment::Center),
        rows[2],
    );
}

fn finish_dino_run(state: &mut SaveFile, flash: &mut Option<Flash>, session: &mut DinoGameSession) {
    let Some(result) = dino::crash(session, &mut state.games.dino) else {
        return;
    };

    if result.is_record || result.unlock_reason.is_some() {
        save::mark_dirty(state);
    }
    save::save_state(state).ok();

    let record_text = if result.is_record {
        " · new record"
    } else {
        ""
    };
    let unlock_text = match result.unlock_reason {
        Some(save::DinoUnlockReason::FirstRecord) => {
            " · unlock trigger queued from the first record"
        }
        Some(save::DinoUnlockReason::Endurance) => " · unlock trigger queued from a 120s+ run",
        None => "",
    };
    let outcome_text = match result.exit_reason {
        crate::dino::integration::DinoExitReason::GameOver => "survived",
    };
    *flash = Some(Flash {
        message: format!(
            "Dino Run: {} {} {} · score {}{}{}",
            state.active_monster().name,
            outcome_text,
            dino::format_duration_ms(result.duration_ms),
            result.score,
            record_text,
            unlock_text
        ),
        kind: FlashKind::Info,
        created_at: Instant::now(),
    });
}

// ── Placeholder ───────────────────────────────────────────────────────────────

fn draw_coming_soon(f: &mut ratatui::Frame, area: Rect, tab: MenuTab) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                tab.label(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Coming soon…",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ])
        .alignment(Alignment::Center),
        chunks[1],
    );
}

// ── Full-screen modals ────────────────────────────────────────────────────────

fn draw_startup_choice(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &SaveFile,
    mode: StartupChoiceMode,
    cursor: usize,
    tick: u64,
) {
    let monster_name = &state.active_monster().name;
    let (headline, subline) = match mode {
        StartupChoiceMode::FirstLaunch => (
            format!("◆  {} has hatched  ◆", monster_name),
            "Pick how you want to play. You can change this later.",
        ),
        StartupChoiceMode::Returning => (
            format!("◆  Welcome back, {}  ◆", monster_name),
            "Sync your monster online or keep coding solo.",
        ),
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(13),
            Constraint::Length(2),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                headline,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(subline, Style::default().fg(Color::DarkGray))),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );

    let cards_area = rows[1];
    let card_width = 38u16.min(cards_area.width / 2);
    let total = card_width * 2 + 4;
    let pad = cards_area.width.saturating_sub(total) / 2;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(pad),
            Constraint::Length(card_width),
            Constraint::Length(4),
            Constraint::Length(card_width),
            Constraint::Min(0),
        ])
        .split(cards_area);

    draw_startup_card(f, cols[1], StartupOption::Online, cursor == 0, tick);
    draw_startup_card(f, cols[3], StartupOption::Offline, cursor == 1, tick);

    f.render_widget(
        Paragraph::new(Span::styled(
            "← →  switch    Enter  confirm    l  online    n  offline    q  quit",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[2],
    );
}

#[derive(Copy, Clone)]
enum StartupOption {
    Online,
    Offline,
}

fn draw_startup_card(
    f: &mut ratatui::Frame,
    area: Rect,
    option: StartupOption,
    selected: bool,
    tick: u64,
) {
    if area.width < 10 || area.height < 6 {
        return;
    }

    let (accent, title, hotkey, headline, perks) = match option {
        StartupOption::Online => (
            Color::LightCyan,
            " ☁  ONLINE  ",
            "L",
            "Sync to the cloud",
            [
                "● GitHub login (device flow)",
                "● Climb the global ladder",
                "● Backup across machines",
            ],
        ),
        StartupOption::Offline => (
            Color::LightYellow,
            " ◐  OFFLINE  ",
            "N",
            "Keep it local",
            [
                "● No account, no network",
                "● Save stays on this machine",
                "● Switch online any time",
            ],
        ),
    };

    let border_color = if selected { accent } else { Color::DarkGray };
    let title_style = Style::default()
        .fg(if selected { accent } else { Color::Gray })
        .add_modifier(Modifier::BOLD);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color).add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }))
        .title(Span::styled(title, title_style))
        .title_alignment(Alignment::Center);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 5 {
        return;
    }

    // Banner row at top — animated for selection
    let banner = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 3.min(inner.height),
    };
    if selected {
        let buf = f.buffer_mut();
        for dy in 0..banner.height {
            for dx in 0..banner.width {
                let n = ((dx as i64 + (tick as i64) / 2) + dy as i64 * 3).rem_euclid(8);
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '◦',
                    3 => '•',
                    _ => ' ',
                };
                let cell = &mut buf[(banner.x + dx, banner.y + dy)];
                cell.set_char(ch);
                cell.set_fg(accent);
                cell.set_bg(Color::Black);
            }
        }
        let icon = match option {
            StartupOption::Online => {
                if (tick / 6) % 2 == 0 {
                    "  ☁  ☁    ☁  "
                } else {
                    "    ☁  ☁  ☁  "
                }
            }
            StartupOption::Offline => "    ◐ ◑ ◒ ◓    ",
        };
        let icon_w = icon.chars().count() as u16;
        let icon_x = banner.x + banner.width.saturating_sub(icon_w) / 2;
        let icon_y = banner.y + banner.height / 2;
        f.buffer_mut().set_string(
            icon_x,
            icon_y,
            icon,
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
    } else {
        let buf = f.buffer_mut();
        for dy in 0..banner.height {
            for dx in 0..banner.width {
                let cell = &mut buf[(banner.x + dx, banner.y + dy)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(15, 15, 18));
            }
        }
        let icon = match option {
            StartupOption::Online => "  ☁  ",
            StartupOption::Offline => "  ◐  ",
        };
        let icon_w = icon.chars().count() as u16;
        let icon_x = banner.x + banner.width.saturating_sub(icon_w) / 2;
        let icon_y = banner.y + banner.height / 2;
        f.buffer_mut().set_string(
            icon_x,
            icon_y,
            icon,
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Rgb(15, 15, 18)),
        );
    }

    // Body region
    let body = Rect {
        x: inner.x,
        y: inner.y + banner.height,
        width: inner.width,
        height: inner.height - banner.height,
    };
    let body_rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(perks.len() as u16),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(body);

    f.render_widget(
        Paragraph::new(Span::styled(
            headline,
            Style::default()
                .fg(if selected { Color::White } else { Color::Gray })
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        body_rows[0],
    );

    let perk_lines: Vec<Line> = perks
        .iter()
        .map(|p| {
            Line::from(Span::styled(
                (*p).to_string(),
                Style::default().fg(if selected { accent } else { Color::DarkGray }),
            ))
        })
        .collect();
    f.render_widget(
        Paragraph::new(perk_lines).alignment(Alignment::Center),
        body_rows[2],
    );

    let action_style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let action_text = if selected {
        format!(
            "  ▶  ENTER to {}  ◀  ",
            match option {
                StartupOption::Online => "go online",
                StartupOption::Offline => "go offline",
            }
        )
    } else {
        format!("  press {}  ", hotkey)
    };
    f.render_widget(
        Paragraph::new(Span::styled(action_text, action_style)).alignment(Alignment::Center),
        body_rows[4],
    );
}

fn draw_login_flow(
    f: &mut ratatui::Frame,
    area: Rect,
    login: &cloud::StartLoginResponse,
    tick: u64,
) {
    let card = center_rect_with_size(area, 60.min(area.width), 18.min(area.height));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(Span::styled(
            " ☁  CONNECTING TO DEVIMON CLOUD  ☁ ",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    let inner = block.inner(card);
    f.render_widget(block, card);

    if inner.height < 8 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    // Step 1: open URL
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " 1 ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Open this URL in your browser"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White)),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            login.verification_uri.clone(),
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );

    // Step 2: enter code
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " 2 ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Enter this code"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White)),
        rows[3],
    );

    // Code pill — pulsing
    let pulse = (tick / 8) % 2 == 0;
    let code_style = if pulse {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightYellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    };
    let code_text = format!("   {}   ", login.user_code);
    f.render_widget(
        Paragraph::new(Span::styled(code_text, code_style)).alignment(Alignment::Center),
        rows[4],
    );

    // Spinner waiting indicator
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = frames[(tick as usize / 2) % frames.len()];
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                spinner,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "Waiting for GitHub authorization…",
                Style::default().fg(Color::Gray),
            ),
        ]))
        .alignment(Alignment::Center),
        rows[6],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Tip: keep this terminal open while you authorize.",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[7],
    );

    f.render_widget(
        Paragraph::new(Span::styled(
            "Esc · cancel",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[9],
    );
}

fn hash3(a: i64, b: i64, c: i64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for v in [a, b, c] {
        h ^= v as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn element_cell(element: Element, x: i64, y: i64, _w: i64, h: i64, tick: u64) -> (char, Style) {
    match element {
        Element::Fire => {
            let t = (tick / 2) as i64;
            let n = (hash3(x, y + t, 11) % 100) as i64;
            let bottom_dist = (h - 1 - y).max(0);
            let heat = 100 - bottom_dist * 80 / h.max(1) - n / 3;
            if heat > 75 {
                (
                    '▲',
                    Style::default()
                        .fg(Color::LightYellow)
                        .bg(Color::Rgb(200, 50, 0)),
                )
            } else if heat > 55 {
                (
                    '▴',
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(140, 25, 0)),
                )
            } else if heat > 35 {
                (
                    '*',
                    Style::default().fg(Color::Red).bg(Color::Rgb(70, 5, 0)),
                )
            } else if heat > 15 {
                (
                    '.',
                    Style::default()
                        .fg(Color::Rgb(180, 60, 0))
                        .bg(Color::Rgb(35, 0, 0)),
                )
            } else {
                (' ', Style::default().bg(Color::Rgb(15, 0, 0)))
            }
        }
        Element::Water => {
            let t = tick as f32 * 0.25;
            let v = (x as f32 * 0.45 + t).sin() + (y as f32 * 0.7 - t * 0.6).cos();
            let n = (hash3(x, y, (tick / 6) as i64) % 100) as i64;
            let val = (v * 30.0) as i64 + 50 + n / 12;
            if val > 78 {
                (
                    '≈',
                    Style::default().fg(Color::White).bg(Color::Rgb(0, 70, 150)),
                )
            } else if val > 58 {
                (
                    '~',
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(0, 45, 115)),
                )
            } else if val > 38 {
                (
                    '-',
                    Style::default().fg(Color::Cyan).bg(Color::Rgb(0, 28, 85)),
                )
            } else if val > 20 {
                (
                    '·',
                    Style::default().fg(Color::Blue).bg(Color::Rgb(0, 18, 60)),
                )
            } else {
                (' ', Style::default().bg(Color::Rgb(0, 12, 45)))
            }
        }
        Element::Grass => {
            let sway = ((x as f32 * 0.35 + tick as f32 * 0.15).sin() * 1.5) as i64;
            let n = (hash3(x + sway, y, 7) % 100) as i64;
            let bottom_dist = (h - 1 - y).max(0);
            if bottom_dist == 0 {
                (
                    '▒',
                    Style::default().fg(Color::Green).bg(Color::Rgb(25, 70, 25)),
                )
            } else if bottom_dist <= 1 + (n % 3) {
                let blade = match (n + sway).rem_euclid(4) {
                    0 => '|',
                    1 => '/',
                    2 => '\\',
                    _ => 'V',
                };
                (
                    blade,
                    Style::default()
                        .fg(Color::LightGreen)
                        .bg(Color::Rgb(0, 30, 5)),
                )
            } else if n < 2 {
                (
                    '❀',
                    Style::default()
                        .fg(Color::LightMagenta)
                        .bg(Color::Rgb(0, 22, 5)),
                )
            } else if n < 8 {
                (
                    '"',
                    Style::default().fg(Color::Green).bg(Color::Rgb(0, 18, 5)),
                )
            } else {
                (' ', Style::default().bg(Color::Rgb(0, 14, 5)))
            }
        }
    }
}

fn render_element_background(f: &mut ratatui::Frame, area: Rect, element: Element, tick: u64) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let buf = f.buffer_mut();
    let w = area.width as i64;
    let h = area.height as i64;
    for dy in 0..area.height {
        for dx in 0..area.width {
            let (ch, style) = element_cell(element, dx as i64, dy as i64, w, h, tick);
            let cell = &mut buf[(area.x + dx, area.y + dy)];
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}

fn render_egg_overlay(
    f: &mut ratatui::Frame,
    area: Rect,
    species: Species,
    selected: bool,
    tick: u64,
) {
    let art = starter_egg_art(species, selected, tick);
    if art.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }
    let art_h = art.len() as u16;
    let art_w = art
        .iter()
        .map(|s| s.chars().count() as u16)
        .max()
        .unwrap_or(0);
    let start_y = area.y + area.height.saturating_sub(art_h) / 2;
    let start_x = area.x + area.width.saturating_sub(art_w) / 2;
    let element = starter_element(species);
    let buf = f.buffer_mut();
    let outline = if selected { Color::White } else { Color::Gray };
    let accent = element_accent(element);
    for (dy, row) in art.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = start_x + dx as u16;
            let y = start_y + dy as u16;
            if x >= area.x + area.width || y >= area.y + area.height {
                continue;
            }
            let fg = match ch {
                'o' | '>' | '<' | '^' | '~' | '.' => accent,
                _ => outline,
            };
            let cell = &mut buf[(x, y)];
            cell.set_char(ch);
            cell.set_fg(fg);
            if selected {
                let style = cell.style().add_modifier(Modifier::BOLD);
                cell.set_style(style);
            }
        }
    }
}

fn draw_onboarding_intro(f: &mut ratatui::Frame, area: Rect, animation_tick: u64) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(20),
            Constraint::Min(0),
        ])
        .split(area);

    let pulse = (animation_tick / 8) % 3;
    let title_color = match pulse {
        0 => Color::LightMagenta,
        1 => Color::LightCyan,
        _ => Color::LightYellow,
    };

    let start_style = if (animation_tick / 10).is_multiple_of(2) {
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    };

    let mut lines = vec![Line::from("")];
    for row in onboarding_title_art() {
        lines.push(Line::from(Span::styled(
            (*row).to_string(),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.extend([
        Line::from(""),
        Line::from(vec![
            Span::styled("◆ ", Style::default().fg(Color::LightMagenta)),
            Span::styled(
                "A monster grows from your code.",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ◆", Style::default().fg(Color::LightMagenta)),
        ]),
        Line::from(Span::styled(
            "Hatch an egg, feed it, train it, climb the ladder.",
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   ▶  PRESS ENTER TO BEGIN  ◀   ",
            start_style,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter · start    Esc · quit",
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        chunks[1],
    );
}

fn draw_onboarding_egg_select(
    f: &mut ratatui::Frame,
    area: Rect,
    cursor: usize,
    animation_tick: u64,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(2),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "◆  CHOOSE YOUR STARTER EGG  ◆",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Three eggs. Three elements. Pick the one that calls to you.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(rows[1]);

    for (index, species) in STARTER_SPECIES.iter().copied().enumerate() {
        let selected = index == cursor;
        let element = starter_element(species);
        let accent = element_accent(element);
        let card_area = cols[index];

        let border_color = if selected { accent } else { Color::DarkGray };
        let title_text = if selected {
            format!(" ◆ {} ◆ ", element_label(element))
        } else {
            format!("  {}  ", element_label(element))
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .title(Span::styled(
                title_text,
                Style::default()
                    .fg(if selected { accent } else { Color::Gray })
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);
        let inner = block.inner(card_area);
        f.render_widget(block, card_area);

        if inner.height < 4 {
            continue;
        }

        let scene_height = inner.height.saturating_sub(4).max(6).min(inner.height);
        let scene = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: scene_height,
        };
        let info = Rect {
            x: inner.x,
            y: inner.y + scene_height,
            width: inner.width,
            height: inner.height - scene_height,
        };

        if selected {
            render_element_background(f, scene, element, animation_tick);
        } else {
            // Subtle deep tint so unselected cards don't look totally flat.
            let buf = f.buffer_mut();
            for dy in 0..scene.height {
                for dx in 0..scene.width {
                    let cell = &mut buf[(scene.x + dx, scene.y + dy)];
                    cell.set_char(' ');
                    cell.set_bg(element_deep(element));
                }
            }
        }

        render_egg_overlay(f, scene, species, selected, animation_tick);

        let info_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(info);

        f.render_widget(
            Paragraph::new(Span::styled(
                starter_species_name(species),
                Style::default()
                    .fg(if selected { Color::White } else { Color::Gray })
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            info_rows[0],
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                element_tagline(element),
                Style::default().fg(if selected { accent } else { Color::DarkGray }),
            ))
            .alignment(Alignment::Center),
            info_rows[1],
        );
        f.render_widget(
            Paragraph::new(starter_species_description(species))
                .style(Style::default().fg(if selected {
                    Color::Gray
                } else {
                    Color::DarkGray
                }))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center),
            info_rows[2],
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled(
            "← →  switch egg    Enter  hatch    Esc  back",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[2],
    );
}

fn draw_onboarding_name(
    f: &mut ratatui::Frame,
    area: Rect,
    species: Species,
    name_input: &str,
    animation_tick: u64,
) {
    let element = starter_element(species);
    let accent = element_accent(element);

    let display_name = if name_input.trim().is_empty() {
        starter_default_name(species)
    } else {
        name_input.trim()
    };

    let card = center_rect_with_size(area, 56.min(area.width), 20.min(area.height));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            format!(" ◆ {} EGG ◆ ", element_label(element)),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    let inner = block.inner(card);
    f.render_widget(block, card);

    if inner.height < 6 {
        return;
    }

    let scene_height = (inner.height / 2).max(6).min(inner.height - 4);
    let scene = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: scene_height,
    };
    render_element_background(f, scene, element, animation_tick);
    render_egg_overlay(f, scene, species, true, animation_tick);

    let form = Rect {
        x: inner.x,
        y: inner.y + scene_height,
        width: inner.width,
        height: inner.height - scene_height,
    };
    let form_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(form);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{} chose you", starter_species_name(species)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        form_rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Give your monster a name",
            Style::default().fg(Color::Gray),
        ))
        .alignment(Alignment::Center),
        form_rows[1],
    );

    let cursor = if (animation_tick / 6).is_multiple_of(2) {
        "▏"
    } else {
        " "
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default().bg(Color::Black)),
            Span::styled(
                format!(
                    " {} ",
                    if name_input.is_empty() {
                        ""
                    } else {
                        name_input
                    }
                ),
                Style::default()
                    .fg(accent)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(cursor, Style::default().fg(accent).bg(Color::Black)),
            Span::styled("  ", Style::default().bg(Color::Black)),
        ]))
        .alignment(Alignment::Center),
        form_rows[2],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("default · {}", display_name),
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        form_rows[3],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter  review    Esc  back",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        form_rows[4],
    );
}

fn draw_onboarding_confirm(
    f: &mut ratatui::Frame,
    area: Rect,
    species: Species,
    name_input: &str,
    confirm_choice: usize,
    animation_tick: u64,
) {
    let element = starter_element(species);
    let accent = element_accent(element);
    let preview = starter_preview_monster(species, name_input);
    let preview_art = display::ascii_art(&preview);

    let modal = center_rect_with_size(area, 56.min(area.width), 22.min(area.height));
    f.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .title(Span::styled(
            " ◆  READY TO HATCH?  ◆ ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    if inner.height < 8 {
        return;
    }

    let scene_height = (inner.height / 2).max(6).min(inner.height - 5);
    let scene = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: scene_height,
    };
    render_element_background(f, scene, element, animation_tick);

    let art_h = preview_art.len() as u16;
    let art_w = preview_art
        .iter()
        .map(|s| s.chars().count() as u16)
        .max()
        .unwrap_or(0);
    let start_y = scene.y + scene.height.saturating_sub(art_h) / 2;
    let start_x = scene.x + scene.width.saturating_sub(art_w) / 2;
    let buf = f.buffer_mut();
    for (dy, row) in preview_art.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let x = start_x + dx as u16;
            let y = start_y + dy as u16;
            if x >= scene.x + scene.width || y >= scene.y + scene.height {
                continue;
            }
            let cell = &mut buf[(x, y)];
            cell.set_char(ch);
            cell.set_fg(Color::White);
            let style = cell.style().add_modifier(Modifier::BOLD);
            cell.set_style(style);
        }
    }

    let info = Rect {
        x: inner.x,
        y: inner.y + scene_height,
        width: inner.width,
        height: inner.height - scene_height,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(info);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("{} the {}", preview.name, starter_species_name(species)),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            element_tagline(element),
            Style::default().fg(Color::Gray),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Starts at level 1. Grows as you code.",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[2],
    );

    let hatch_style = if confirm_choice == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
    };
    let back_style = if confirm_choice == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ▶ HATCH ◀  ", hatch_style),
            Span::raw("    "),
            Span::styled("  BACK  ", back_style),
        ]))
        .alignment(Alignment::Center),
        rows[3],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "← →  choose    Enter  confirm    Esc  back",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Center),
        rows[4],
    );
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Filled/empty block string for a need value.
fn bar_chars(value: f32, width: usize) -> String {
    let filled = ((value / 100.0) * width as f32).round() as usize;
    let filled = filled.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn mini_bar_line(label: &str, value: f32) -> Line<'static> {
    const W: usize = 8;
    let filled = ((value / 100.0) * W as f32).round() as usize;
    let filled = filled.min(W);
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("█".repeat(filled), Style::default().fg(need_color(value))),
        Span::styled("░".repeat(W - filled), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {:>3.0}", value),
            Style::default().fg(Color::White),
        ),
    ])
}

fn center_rect(area: Rect, percent: u16) -> Rect {
    let margin = (100 - percent) / 2;
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(margin),
            Constraint::Percentage(percent),
            Constraint::Percentage(margin),
        ])
        .split(area)[1]
}

fn center_rect_with_size(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn render_xp_gauge(f: &mut ratatui::Frame, area: Rect, monster: &Monster) {
    let ratio = (monster.xp as f64 / monster.xp_to_next() as f64).clamp(0.0, 1.0);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .ratio(ratio)
        .label(Span::styled(
            format!("XP  {}/{}", monster.xp, monster.xp_to_next()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(gauge, area);
}

fn need_color(value: f32) -> Color {
    if value >= 60.0 {
        Color::Green
    } else if value >= 30.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}
