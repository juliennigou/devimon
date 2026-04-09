use crate::actions;
use crate::display::{self, MoodState};
use crate::monster::Monster;
use crate::save;
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
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
};
use std::io::{self, Stdout};
use std::thread;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(500);
const FLASH_DURATION: Duration = Duration::from_secs(3);

enum AppState {
    Onboarding { name_input: String },
    Running { monster: Monster, flash: Option<Flash> },
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

pub fn run() -> io::Result<()> {
    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Start the file watcher in a background thread so the UI picks up
    // real developer activity automatically. Errors are non-fatal — the
    // UI still works without a watcher.
    if let Ok(cwd) = std::env::current_dir() {
        thread::spawn(move || {
            let _ = watcher::watch_silent(&cwd);
        });
    }

    // Run the main loop.
    let result = run_app(&mut terminal);

    // Terminal teardown.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut state = initial_state()?;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &state))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut state, key.code, key.modifiers)?;
                }
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            tick(&mut state)?;
            last_tick = Instant::now();
        }

        if matches!(state, AppState::Quit) {
            // Final save on clean exit.
            if let AppState::Running { monster, .. } = &state {
                let _ = save::save(monster);
            }
            break;
        }
    }
    Ok(())
}

fn initial_state() -> io::Result<AppState> {
    match save::load()? {
        Some(monster) => Ok(AppState::Running {
            monster,
            flash: Some(Flash {
                message: "Bon retour ! Ton monstre t'attendait.".to_string(),
                kind: FlashKind::Info,
                created_at: Instant::now(),
            }),
        }),
        None => Ok(AppState::Onboarding {
            name_input: String::new(),
        }),
    }
}

fn tick(state: &mut AppState) -> io::Result<()> {
    if let AppState::Running { monster, flash } = state {
        // Drain the XP event queue (populated by the background watcher or
        // by an external `devimon watch` process).
        let xp_gained = xp::drain_and_apply(monster).unwrap_or(0);
        monster.apply_decay();
        if let Some(new_stage) = monster.check_evolution() {
            *flash = Some(Flash {
                message: format!("✨ {} a évolué — {} !", monster.name, new_stage.label()),
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
        // Save on any state that was mutated by drain/decay/evolution.
        save::save(monster).ok();
    }
    Ok(())
}

fn handle_key(state: &mut AppState, code: KeyCode, mods: KeyModifiers) -> io::Result<()> {
    // Global Ctrl+C quit.
    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c')) {
        *state = AppState::Quit;
        return Ok(());
    }

    match state {
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
                let monster = Monster::spawn(name.clone());
                save::save(&monster).ok();
                *state = AppState::Running {
                    monster,
                    flash: Some(Flash {
                        message: format!("🥚 {} est né !", name),
                        kind: FlashKind::Success,
                        created_at: Instant::now(),
                    }),
                };
            }
            KeyCode::Esc => *state = AppState::Quit,
            _ => {}
        },
        AppState::Running { monster, flash } => match code {
            KeyCode::Char('q') | KeyCode::Esc => *state = AppState::Quit,
            KeyCode::Char('f') => {
                let result = actions::feed(monster);
                *flash = Some(make_flash(result));
                save::save(monster).ok();
            }
            KeyCode::Char('p') => {
                let result = actions::play(monster);
                *flash = Some(make_flash(result));
                save::save(monster).ok();
            }
            KeyCode::Char('r') => {
                let result = actions::rest(monster);
                *flash = Some(make_flash(result));
                save::save(monster).ok();
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

// ---------- Rendering ----------

fn draw(f: &mut ratatui::Frame, state: &AppState) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Devimon 🐾 ")
        .title_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = outer.inner(f.area());
    f.render_widget(outer, f.area());

    match state {
        AppState::Onboarding { name_input } => draw_onboarding(f, inner, name_input),
        AppState::Running { monster, flash } => draw_running(f, inner, monster, flash),
        AppState::Quit => {}
    }
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

    let lines = vec![
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
    ];

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, chunks[1]);
}

fn draw_running(f: &mut ratatui::Frame, area: Rect, monster: &Monster, flash: &Option<Flash>) {
    // Split content from footer.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let content = rows[0];
    let footer_area = rows[1];

    // Monster panel spans the full content width (art is centered within it).
    draw_monster_panel(f, content, monster, flash);

    // Stats panel sits in the top-right corner as an overlay.
    draw_stats_panel(f, content, monster);

    // Footer.
    draw_footer(f, footer_area);
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
            Constraint::Length(1), // name · lv · stage  (above art)
            Constraint::Length(1), // spacer
            Constraint::Length(5), // big ASCII art
            Constraint::Length(1), // spacer
            Constraint::Length(1), // XP gauge
            Constraint::Length(1), // spacer
            Constraint::Length(1), // personality
            Constraint::Length(1), // flash
            Constraint::Min(0),    // filler
        ])
        .split(area);

    // Name · lv · stage — centered above the art.
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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

    // Big ASCII art — centered.
    let art = display::ascii_art_big(monster);
    let art_lines: Vec<Line> = art
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
    f.render_widget(
        Paragraph::new(art_lines).alignment(Alignment::Center),
        chunks[2],
    );

    // XP gauge — centered at 55% of the total width.
    let xp_area = center_rect(chunks[4], 55);
    render_xp_gauge(f, xp_area, monster);

    // Personality line.
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

    // Ephemeral flash message.
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

/// Compact stats block overlaid in the top-right corner of `area`.
fn draw_stats_panel(f: &mut ratatui::Frame, area: Rect, monster: &Monster) {
    const PANEL_W: u16 = 22; // 2 borders + 20 inner
    const PANEL_H: u16 = 5; // 2 borders + 3 bar rows

    if area.width < PANEL_W || area.height < PANEL_H {
        return;
    }

    let rect = Rect {
        x: area.x + area.width - PANEL_W,
        y: area.y,
        width: PANEL_W,
        height: PANEL_H,
    };

    // Wipe whatever was drawn underneath.
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Stats ")
        .title_style(Style::default().fg(Color::DarkGray))
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    // Three rows, one per stat.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(mini_bar("Faim   ", monster.hunger)), rows[0]);
    f.render_widget(Paragraph::new(mini_bar("Énergie", monster.energy)), rows[1]);
    f.render_widget(Paragraph::new(mini_bar("Moral  ", monster.mood)), rows[2]);
}

/// One line: "Faim    ████░░░░  80"  (exactly 20 display chars inside the panel).
fn mini_bar(label: &str, value: f32) -> Line<'static> {
    const BAR_W: usize = 8;
    let filled = ((value / 100.0) * BAR_W as f32).round() as usize;
    let filled = filled.min(BAR_W);
    let empty = BAR_W - filled;
    let color = need_color(value);

    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {:>3.0}", value),
            Style::default().fg(Color::White),
        ),
    ])
}

fn draw_footer(f: &mut ratatui::Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" f ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" feed   "),
        Span::styled(" p ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" play   "),
        Span::styled(" r ", Style::default().bg(Color::DarkGray).fg(Color::Green)),
        Span::raw(" rest   "),
        Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::Red)),
        Span::raw(" quit"),
    ]);
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

/// Returns a horizontally centered sub-rect that is `percent`% of `area`'s width.
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
    let label = format!("XP  {}/{}", monster.xp, monster.xp_to_next());
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .ratio(ratio)
        .label(Span::styled(
            label,
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
