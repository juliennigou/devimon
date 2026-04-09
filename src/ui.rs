use crate::actions;
use crate::cloud::{self, PollLoginStatus};
use crate::display::{self, MoodState};
use crate::monster::Monster;
use crate::save::{self, SaveFile};
use crate::watcher;
use crate::xp;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, block::Title},
};
use std::io::{self, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(500);
const FLASH_DURATION: Duration = Duration::from_secs(3);
const SYNC_RATE: Duration = Duration::from_secs(20);

// ── Menu ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuTab {
    Home,
    Collection,
    Games,
    Account,
    Team,
    Settings,
}

impl MenuTab {
    fn label(self) -> &'static str {
        match self {
            MenuTab::Home => "Home",
            MenuTab::Collection => "Collection",
            MenuTab::Games => "Games",
            MenuTab::Account => "Account",
            MenuTab::Team => "Team",
            MenuTab::Settings => "Settings",
        }
    }
}

const MENU_ITEMS: &[MenuTab] = &[
    MenuTab::Home,
    MenuTab::Collection,
    MenuTab::Games,
    MenuTab::Account,
    MenuTab::Team,
    MenuTab::Settings,
];

// ── App state ─────────────────────────────────────────────────────────────────

enum AppState {
    StartupChoice {
        state: SaveFile,
    },
    Onboarding {
        name_input: String,
    },
    LoginFlow {
        state: SaveFile,
        login: cloud::StartLoginResponse,
        result_rx: mpsc::Receiver<Result<cloud::AccountEnvelope, String>>,
    },
    Running {
        state: SaveFile,
        flash: Option<Flash>,
        last_sync_attempt: Instant,
        selected_tab: MenuTab,
        collection_cursor: usize,
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut app = initial_state()?;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key.code, key.modifiers)?;
                }
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            tick(&mut app)?;
            last_tick = Instant::now();
        }

        if matches!(app, AppState::Quit) {
            break;
        }
    }
    Ok(())
}

fn initial_state() -> io::Result<AppState> {
    match save::load_state()? {
        None => Ok(AppState::Onboarding {
            name_input: String::new(),
        }),
        Some(state) => {
            if state.cloud.account.is_none() {
                Ok(AppState::StartupChoice { state })
            } else {
                Ok(AppState::Running {
                    state,
                    flash: Some(Flash {
                        message: "Bon retour ! Ton monstre t'attendait.".to_string(),
                        kind: FlashKind::Info,
                        created_at: Instant::now(),
                    }),
                    last_sync_attempt: Instant::now() - SYNC_RATE,
                    selected_tab: MenuTab::Home,
                    collection_cursor: 0,
                })
            }
        }
    }
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
                    let new_state = state.clone();
                    *app = AppState::Running {
                        state: new_state,
                        flash: Some(Flash {
                            message: format!("Logged in as @{}!", username),
                            kind: FlashKind::Success,
                            created_at: Instant::now(),
                        }),
                        last_sync_attempt: Instant::now() - SYNC_RATE,
                        selected_tab: MenuTab::Home,
                        collection_cursor: 0,
                    };
                }
                Err(e) => {
                    let new_state = state.clone();
                    *app = AppState::Running {
                        state: new_state,
                        flash: Some(Flash {
                            message: format!("Login failed: {}", e),
                            kind: FlashKind::Error,
                            created_at: Instant::now(),
                        }),
                        last_sync_attempt: Instant::now() - SYNC_RATE,
                        selected_tab: MenuTab::Home,
                        collection_cursor: 0,
                    };
                }
            }
        }
        return Ok(());
    }

    if let AppState::Running {
        state,
        flash,
        last_sync_attempt,
        ..
    } = app
    {
        let idx = state.active_monster_idx();
        let xp_gained = xp::drain_and_apply(&mut state.monsters[idx]).unwrap_or(0);
        if xp_gained > 0 {
            save::mark_dirty(state);
        }

        state.monsters[idx].apply_decay();
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

// ── Input ─────────────────────────────────────────────────────────────────────

fn persist_and_quit(app: &mut AppState) {
    let to_save: Option<SaveFile> = match app {
        AppState::Running { state, .. } => Some(state.clone()),
        AppState::LoginFlow { state, .. } => Some(state.clone()),
        AppState::StartupChoice { state } => Some(state.clone()),
        _ => None,
    };
    if let Some(ref state) = to_save {
        let _ = save::save_state(state);
    }
    *app = AppState::Quit;
}

fn handle_key(app: &mut AppState, code: KeyCode, mods: KeyModifiers) -> io::Result<()> {
    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
        persist_and_quit(app);
        return Ok(());
    }

    match app {
        AppState::StartupChoice { state } => match code {
            KeyCode::Char('l') => {
                let state_owned = state.clone();
                match cloud::start_login() {
                    Ok(login) => {
                        let (tx, rx) = mpsc::channel();
                        spawn_login_poller(login.login_id.clone(), login.interval_seconds, tx);
                        *app = AppState::LoginFlow {
                            state: state_owned,
                            login,
                            result_rx: rx,
                        };
                    }
                    Err(e) => {
                        *app = AppState::Running {
                            state: state_owned,
                            flash: Some(Flash {
                                message: format!("Impossible de démarrer la connexion: {}", e),
                                kind: FlashKind::Error,
                                created_at: Instant::now(),
                            }),
                            last_sync_attempt: Instant::now() - SYNC_RATE,
                            selected_tab: MenuTab::Home,
                            collection_cursor: 0,
                        };
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char(' ') => {
                let state_owned = state.clone();
                *app = AppState::Running {
                    state: state_owned,
                    flash: None,
                    last_sync_attempt: Instant::now() - SYNC_RATE,
                    selected_tab: MenuTab::Home,
                    collection_cursor: 0,
                };
            }
            KeyCode::Esc | KeyCode::Char('q') => persist_and_quit(app),
            _ => {}
        },

        AppState::LoginFlow { state, .. } => match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                let state_owned = state.clone();
                *app = AppState::Running {
                    state: state_owned,
                    flash: Some(Flash {
                        message: "Connexion annulée.".to_string(),
                        kind: FlashKind::Info,
                        created_at: Instant::now(),
                    }),
                    last_sync_attempt: Instant::now() - SYNC_RATE,
                    selected_tab: MenuTab::Home,
                    collection_cursor: 0,
                };
            }
            _ => {}
        },

        AppState::Onboarding { name_input } => match code {
            KeyCode::Char(c) if name_input.chars().count() < 20 => name_input.push(c),
            KeyCode::Backspace => {
                name_input.pop();
            }
            KeyCode::Enter => {
                let name = if name_input.trim().is_empty() {
                    "Devi".to_string()
                } else {
                    name_input.trim().to_string()
                };
                let state = SaveFile::new(Monster::spawn(name.clone()));
                save::save_state(&state).ok();
                *app = AppState::StartupChoice { state };
            }
            KeyCode::Esc => *app = AppState::Quit,
            _ => {}
        },

        AppState::Running {
            state,
            flash,
            last_sync_attempt,
            selected_tab,
            collection_cursor,
        } => match code {
            KeyCode::Char('q') | KeyCode::Esc => persist_and_quit(app),

            // ── Navigation ────────────────────────────────────────────────
            // Left/Right → switch sidebar tab
            KeyCode::Left => {
                let idx = MENU_ITEMS
                    .iter()
                    .position(|&t| t == *selected_tab)
                    .unwrap_or(0);
                if idx > 0 {
                    *selected_tab = MENU_ITEMS[idx - 1];
                    *collection_cursor = 0;
                }
            }
            KeyCode::Right => {
                let idx = MENU_ITEMS
                    .iter()
                    .position(|&t| t == *selected_tab)
                    .unwrap_or(0);
                if idx + 1 < MENU_ITEMS.len() {
                    *selected_tab = MENU_ITEMS[idx + 1];
                    *collection_cursor = 0;
                }
            }
            // Up/Down → move within current tab's content list
            KeyCode::Up if *selected_tab == MenuTab::Collection => {
                if *collection_cursor > 0 {
                    *collection_cursor -= 1;
                }
            }
            KeyCode::Down if *selected_tab == MenuTab::Collection => {
                if *collection_cursor + 1 < state.monsters.len() {
                    *collection_cursor += 1;
                }
            }
            KeyCode::Enter if *selected_tab == MenuTab::Collection => {
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
                let message = match sync.leaderboard_rank {
                    Some(rank) => format!("☁️ Sync ok — rang #{}", rank),
                    None => "☁️ Sync ok".to_string(),
                };
                *flash = Some(Flash {
                    message,
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

fn draw(f: &mut ratatui::Frame, app: &AppState) {
    let online = if let AppState::Running { state, .. } = app {
        state.cloud.account.is_some()
    } else {
        false
    };

    let (status_label, status_color) = if online {
        (" ● Online ", Color::Green)
    } else {
        (" ● Offline ", Color::DarkGray)
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
        AppState::StartupChoice { state } => draw_startup_choice(f, inner, state),
        AppState::Onboarding { name_input } => draw_onboarding(f, inner, name_input),
        AppState::LoginFlow { login, .. } => draw_login_flow(f, inner, login),
        AppState::Running {
            state,
            flash,
            selected_tab,
            collection_cursor,
            ..
        } => draw_running(f, inner, state, flash, *selected_tab, *collection_cursor),
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
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(area);

    draw_sidebar(f, cols[0], selected_tab, state);
    draw_content(f, cols[1], state, flash, selected_tab, collection_cursor);
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn draw_sidebar(f: &mut ratatui::Frame, area: Rect, selected: MenuTab, state: &SaveFile) {
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

    // Menu items
    let mut lines: Vec<Line> = vec![Line::from("")];
    for &tab in MENU_ITEMS {
        if tab == selected {
            lines.push(Line::from(vec![
                Span::styled(
                    " ▶  ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    tab.label(),
                    Style::default()
                        .fg(Color::White)
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
    f.render_widget(
        Paragraph::new(Span::styled(
            " ↑↓ navigate",
            Style::default().fg(Color::DarkGray),
        )),
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
) {
    match selected_tab {
        MenuTab::Home => draw_home(f, area, state, flash),
        MenuTab::Collection => draw_collection(f, area, state, collection_cursor, flash),
        tab => draw_coming_soon(f, area, tab),
    }
}

// ── Home ──────────────────────────────────────────────────────────────────────

fn draw_home(f: &mut ratatui::Frame, area: Rect, state: &SaveFile, flash: &Option<Flash>) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(area);

    draw_monster_panel(f, rows[0], state.active_monster(), flash);
    draw_stats_panel(f, rows[0], state.active_monster());
    draw_footer(f, rows[1], state);
}

fn draw_monster_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    monster: &Monster,
    flash: &Option<Flash>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
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

    let art: Vec<Line> = display::ascii_art_big(monster)
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
    f.render_widget(Paragraph::new(art).alignment(Alignment::Center), chunks[2]);

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
        Line::from(vec![
            Span::styled("☁ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("@{} · {}", account.username, suffix),
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
            i == cursor,
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
    let hint = if n > 1 {
        " ↑↓ select  ·  Enter set main  ·  ←→ menu"
    } else {
        " spawn more monsters with `devimon spawn <name>`"
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

fn draw_startup_choice(f: &mut ratatui::Frame, area: Rect, state: &SaveFile) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(13),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("👋 Bon retour, {} !", state.active_monster().name),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Rejoindre le classement en ligne ?",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" l ", Style::default().bg(Color::DarkGray).fg(Color::Cyan)),
                Span::styled("  Login via GitHub", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" n ", Style::default().bg(Color::DarkGray).fg(Color::White)),
                Span::styled("  Rester hors ligne", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[Entrée] hors ligne    [q] quitter",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        chunks[1],
    );
}

fn draw_login_flow(f: &mut ratatui::Frame, area: Rect, login: &cloud::StartLoginResponse) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(13),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "☁️  Connexion à Devimon Cloud",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Ouvre cette URL dans ton navigateur :",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                login.verification_uri.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Code :  ", Style::default().fg(Color::White)),
                Span::styled(
                    login.user_code.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "En attente d'autorisation…",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[q] Annuler",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        chunks[1],
    );
}

fn draw_onboarding(f: &mut ratatui::Frame, area: Rect, name_input: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(14),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "🥚 Bienvenue dans Devimon",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Ton compagnon de terminal va naître."),
            Line::from("Il grandira avec ton travail réel."),
            Line::from(""),
            Line::from(Span::styled(
                "Quel est son nom ?",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("> {}_", name_input),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "[Entrée] confirmer    [Échap] quitter",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center),
        chunks[1],
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
