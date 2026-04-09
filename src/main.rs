mod actions;
mod display;
mod monster;
mod save;
mod ui;
mod watcher;
mod xp;

use clap::{Parser, Subcommand};
use colored::*;
use monster::Monster;
use std::env;
use std::process;

#[derive(Parser)]
#[command(name = "devimon", about = "Devimon — your terminal companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive TUI (default when no subcommand is given)
    Ui,
    /// Spawn a new monster
    Spawn {
        /// Name of the monster
        name: Option<String>,
    },
    /// Show the monster's current state
    Status,
    /// Feed your monster
    Feed,
    /// Play with your monster
    Play,
    /// Let your monster rest
    Rest,
    /// Start the file watcher in the current directory
    Watch,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {}", "error:".red().bold(), e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command.unwrap_or(Commands::Ui) {
        Commands::Ui => ui::run().map_err(|e| e.to_string()),
        Commands::Spawn { name } => cmd_spawn(name),
        Commands::Status => cmd_status(),
        Commands::Feed => cmd_feed(),
        Commands::Play => cmd_play(),
        Commands::Rest => cmd_rest(),
        Commands::Watch => cmd_watch(),
    }
}

fn load_or_err() -> Result<Monster, String> {
    match save::load().map_err(|e| e.to_string())? {
        Some(m) => Ok(m),
        None => Err("no monster found — run `pet spawn [name]` first.".into()),
    }
}

/// Load the monster, then drain events + apply decay + check evolution.
/// Returns the monster and the XP that was applied from the event queue.
fn load_and_tick() -> Result<(Monster, u32), String> {
    let mut monster = load_or_err()?;
    let xp_gained = xp::drain_and_apply(&mut monster).map_err(|e| e.to_string())?;
    monster.apply_decay();
    if let Some(new_stage) = monster.check_evolution() {
        println!(
            "{}",
            format!(
                "✨ {} a évolué — {} !",
                monster.name,
                new_stage.label()
            )
            .bright_magenta()
            .bold()
        );
    }
    Ok((monster, xp_gained))
}

fn cmd_spawn(name: Option<String>) -> Result<(), String> {
    if save::load().map_err(|e| e.to_string())?.is_some() {
        return Err(
            "a monster already exists. Delete ~/.devimon/save.json to start over.".into(),
        );
    }
    let name = name.unwrap_or_else(|| "Devi".to_string());
    let monster = Monster::spawn(name.clone());
    save::save(&monster).map_err(|e| e.to_string())?;
    println!(
        "🥚 {} est né ! Prends-en soin.",
        name.bright_magenta().bold()
    );
    display::render_status(&monster, 0);
    Ok(())
}

fn cmd_status() -> Result<(), String> {
    let (monster, xp_gained) = load_and_tick()?;
    save::save(&monster).map_err(|e| e.to_string())?;
    display::render_status(&monster, xp_gained);
    Ok(())
}

fn cmd_feed() -> Result<(), String> {
    let (mut monster, xp_gained) = load_and_tick()?;
    let msg = actions::feed(&mut monster)?;
    save::save(&monster).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(&monster, xp_gained);
    Ok(())
}

fn cmd_play() -> Result<(), String> {
    let (mut monster, xp_gained) = load_and_tick()?;
    let msg = actions::play(&mut monster)?;
    save::save(&monster).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(&monster, xp_gained);
    Ok(())
}

fn cmd_rest() -> Result<(), String> {
    let (mut monster, xp_gained) = load_and_tick()?;
    let msg = actions::rest(&mut monster)?;
    save::save(&monster).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(&monster, xp_gained);
    Ok(())
}

fn cmd_watch() -> Result<(), String> {
    // Ensure a monster exists before we start buffering events.
    load_or_err()?;
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    watcher::watch(&cwd).map_err(|e| e.to_string())?;
    Ok(())
}
