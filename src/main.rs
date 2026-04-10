mod actions;
mod cloud;
mod dino;
mod display;
mod monster;
mod save;
mod ui;
mod watcher;
mod xp;

use clap::{Parser, Subcommand};
use cloud::{PollLoginStatus, SyncResponse};
use colored::*;
use save::SaveFile;
use std::env;
use std::process;
use std::thread;
use std::time::Duration;

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
        /// Species: devimon (default) or dragon
        #[arg(long, default_value = "devimon")]
        species: String,
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
    /// Link your local monster to an online account
    Login,
    /// Clear the local online session
    Logout,
    /// Show the connected online account
    Whoami,
    /// Upload the current monster state to the leaderboard backend
    Sync,
    /// Update Devimon to the latest version
    Update,
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
        Commands::Spawn { name, species } => cmd_spawn(name, species),
        Commands::Status => cmd_status(),
        Commands::Feed => cmd_feed(),
        Commands::Play => cmd_play(),
        Commands::Rest => cmd_rest(),
        Commands::Watch => cmd_watch(),
        Commands::Login => cmd_login(),
        Commands::Logout => cmd_logout(),
        Commands::Whoami => cmd_whoami(),
        Commands::Sync => cmd_sync(),
        Commands::Update => cmd_update(),
    }
}

fn load_state_or_err() -> Result<SaveFile, String> {
    match save::load_state().map_err(|e| e.to_string())? {
        Some(state) => Ok(state),
        None => Err("no monster found — run `devimon spawn [name]` first.".into()),
    }
}

/// Load the monster, then drain events + apply decay + check evolution.
/// Returns the full local state and the XP that was applied from the event queue.
fn load_and_tick() -> Result<(SaveFile, u32), String> {
    let mut state = load_state_or_err()?;
    let idx = state.active_monster_idx();
    let xp_gained = xp::drain_and_apply(&mut state.monsters[idx]).map_err(|e| e.to_string())?;
    if xp_gained > 0 {
        save::mark_dirty(&mut state);
    }
    state.monsters[idx].apply_decay();
    if let Some(new_stage) = state.monsters[idx].check_evolution() {
        let name = state.monsters[idx].name.clone();
        println!(
            "{}",
            format!("✨ {} a évolué — {} !", name, new_stage.label())
                .bright_magenta()
                .bold()
        );
        save::mark_dirty(&mut state);
    }
    Ok((state, xp_gained))
}

fn print_sync_status(sync: &SyncResponse) {
    println!(
        "{}",
        format!("☁️  Sync complete — monster id {}", sync.monster_id)
            .bright_cyan()
            .bold()
    );
    if let Some(rank) = sync.leaderboard_rank {
        println!(
            "{}",
            format!("🏆 Current leaderboard rank: #{}", rank).bright_yellow()
        );
    }
}

fn maybe_sync_after_local_change(state: &mut SaveFile) {
    if state.cloud.account.is_none() || !state.cloud.sync_dirty {
        return;
    }

    match cloud::sync_state(state) {
        Ok(sync) => {
            let _ = save::save_state(state);
            print_sync_status(&sync);
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                "warn:".yellow().bold(),
                format!("local changes were saved, but cloud sync failed: {}", e).yellow()
            );
        }
    }
}

fn cmd_spawn(name: Option<String>, species_str: String) -> Result<(), String> {
    let name = name.unwrap_or_else(|| "Devi".to_string());
    let species = match species_str.to_lowercase().as_str() {
        "dragon" => monster::Species::Dragon,
        "devimon" => monster::Species::Devimon,
        other => {
            return Err(format!(
                "unknown species '{}' — try: devimon, dragon",
                other
            ));
        }
    };
    match save::load_state().map_err(|e| e.to_string())? {
        None => {
            let mut m = monster::Monster::spawn(name.clone());
            m.species = species;
            let state = SaveFile::new(m);
            save::save_state(&state).map_err(|e| e.to_string())?;
            println!(
                "🥚 {} est né ! Prends-en soin.",
                name.bright_magenta().bold()
            );
            display::render_status(state.active_monster(), 0);
        }
        Some(mut state) => {
            if !state.is_name_available(&name) {
                return Err(format!(
                    "a monster named '{}' already exists in your collection.",
                    name
                ));
            }
            let mut m = monster::Monster::spawn(name.clone());
            m.species = species;
            state.monsters.push(m);
            save::save_state(&state).map_err(|e| e.to_string())?;
            println!(
                "🥚 {} a rejoint ta collection !",
                name.bright_magenta().bold()
            );
            println!(
                "{}",
                "Open `devimon` → Collection to set it as main.".bright_black()
            );
        }
    }
    Ok(())
}

fn cmd_status() -> Result<(), String> {
    let (mut state, xp_gained) = load_and_tick()?;
    save::save_state(&state).map_err(|e| e.to_string())?;
    display::render_status(state.active_monster(), xp_gained);
    if let Some(account) = &state.cloud.account {
        println!(
            "  {}",
            format!("Cloud: linked as @{}", account.username).bright_cyan()
        );
        if let Some(monster_id) = &state.cloud.monster_id {
            println!("  {}", format!("Monster ID: {}", monster_id).bright_black());
        }
    }
    println!();
    maybe_sync_after_local_change(&mut state);
    Ok(())
}

fn cmd_feed() -> Result<(), String> {
    let (mut state, xp_gained) = load_and_tick()?;
    let msg = actions::feed(state.active_monster_mut())?;
    save::mark_dirty(&mut state);
    save::save_state(&state).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(state.active_monster(), xp_gained);
    maybe_sync_after_local_change(&mut state);
    Ok(())
}

fn cmd_play() -> Result<(), String> {
    let (mut state, xp_gained) = load_and_tick()?;
    let msg = actions::play(state.active_monster_mut())?;
    save::mark_dirty(&mut state);
    save::save_state(&state).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(state.active_monster(), xp_gained);
    maybe_sync_after_local_change(&mut state);
    Ok(())
}

fn cmd_rest() -> Result<(), String> {
    let (mut state, xp_gained) = load_and_tick()?;
    let msg = actions::rest(state.active_monster_mut())?;
    save::mark_dirty(&mut state);
    save::save_state(&state).map_err(|e| e.to_string())?;
    println!("{}", msg.bright_green());
    display::render_status(state.active_monster(), xp_gained);
    maybe_sync_after_local_change(&mut state);
    Ok(())
}

fn cmd_watch() -> Result<(), String> {
    // Ensure a monster exists before we start buffering events.
    load_state_or_err()?;
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    watcher::watch(&cwd).map_err(|e| e.to_string())?;
    Ok(())
}

fn cmd_login() -> Result<(), String> {
    let mut state = load_state_or_err()?;

    if let Some(account) = &state.cloud.account {
        if cloud::validate_session(account).is_ok() {
            println!(
                "{}",
                format!("Already logged in as @{}.", account.username)
                    .bright_green()
                    .bold()
            );
            return Ok(());
        }
    }

    let login = cloud::start_login()?;
    println!("{}", "Connect Devimon to GitHub".bright_magenta().bold());
    println!("Open: {}", login.verification_uri.underline());
    println!(
        "{} {}",
        "Enter code:".bright_yellow().bold(),
        login.user_code.bright_yellow().bold()
    );
    println!(
        "{}",
        format!("Waiting for approval until {}...", login.expires_at).bright_black()
    );

    let mut poll_every = Duration::from_secs(login.interval_seconds.max(1));
    loop {
        let response = cloud::poll_login(&login.login_id)?;
        match response.status {
            PollLoginStatus::Pending => {
                thread::sleep(poll_every);
                if let Some(next_interval) = response.interval_seconds {
                    poll_every = Duration::from_secs(next_interval.max(1));
                }
            }
            PollLoginStatus::Complete => {
                let account = response
                    .account
                    .ok_or_else(|| "login completed without account data".to_string())?;
                let username = account.username.clone();
                state.cloud.account = Some(account.into());
                save::mark_dirty(&mut state);
                let sync = cloud::sync_state(&mut state)?;
                save::save_state(&state).map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    format!("Logged in as @{}.", username).bright_green().bold()
                );
                print_sync_status(&sync);
                return Ok(());
            }
            PollLoginStatus::Expired | PollLoginStatus::Denied => {
                return Err(response
                    .message
                    .unwrap_or_else(|| "login was not approved.".to_string()));
            }
        }
    }
}

fn cmd_logout() -> Result<(), String> {
    let mut state = load_state_or_err()?;
    let username = state
        .cloud
        .account
        .as_ref()
        .map(|account| account.username.clone());
    save::clear_session(&mut state);
    save::save_state(&state).map_err(|e| e.to_string())?;
    if let Some(username) = username {
        println!(
            "{}",
            format!("Logged out @{} locally.", username).bright_green()
        );
    } else {
        println!("{}", "No active cloud session to clear.".bright_black());
    }
    Ok(())
}

fn cmd_whoami() -> Result<(), String> {
    let state = load_state_or_err()?;
    let account = state
        .cloud
        .account
        .as_ref()
        .ok_or_else(|| "not logged in — run `devimon login` first.".to_string())?;
    let me = cloud::fetch_me(account)?;
    println!("{}", "Devimon Cloud".bright_magenta().bold());
    println!("  Username: @{}", me.username);
    println!("  Account ID: {}", me.account_id);
    println!("  Device ID: {}", state.cloud.device_id);
    if let Some(monster_id) = me.monster_id.or_else(|| state.cloud.monster_id.clone()) {
        println!("  Monster ID: {}", monster_id);
    }
    Ok(())
}

const GITHUB_REPO: &str = "juliennigou/devimon";

fn cmd_update() -> Result<(), String> {
    let current = env!("CARGO_PKG_VERSION");
    println!("{}", "Checking for updates…".bright_cyan());

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let resp = client
        .get(&api_url)
        .header("User-Agent", "devimon-updater")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("failed to reach GitHub: {}", e))?;

    if resp.status().as_u16() == 404 {
        return Err("no releases published yet — try again after the next tag is pushed.".into());
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }

    let release: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to parse release info: {}", e))?;

    let tag = release["tag_name"]
        .as_str()
        .ok_or("release has no tag_name")?
        .trim_start_matches('v');

    if tag == current {
        println!(
            "{}",
            format!("Already on the latest version ({}).", current)
                .bright_green()
                .bold()
        );
        return Ok(());
    }

    println!(
        "{}",
        format!("New version available: {} → {}", current, tag).bright_yellow()
    );

    let asset_name = platform_asset_name()?;

    let assets = release["assets"]
        .as_array()
        .ok_or("release has no assets")?;

    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(asset_name))
        .ok_or_else(|| {
            format!(
                "no pre-built binary for your platform ({}). \
                 Build from source with: cargo install --git https://github.com/{} --locked --force",
                asset_name, GITHUB_REPO
            )
        })?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("asset has no download URL")?;

    println!("{}", format!("Downloading {}…", asset_name).bright_cyan());

    let bytes = client
        .get(download_url)
        .header("User-Agent", "devimon-updater")
        .send()
        .and_then(|r| r.bytes())
        .map_err(|e| format!("download failed: {}", e))?;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("cannot locate current binary: {}", e))?;

    // Write to a temp file beside the binary, then atomically rename.
    let tmp = current_exe.with_extension("update-tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("failed to write update to disk: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to set binary permissions: {}", e))?;
    }

    std::fs::rename(&tmp, &current_exe)
        .map_err(|e| format!("failed to replace binary (try with sudo?): {}", e))?;

    println!(
        "{}",
        format!(
            "Updated to {}! Restart devimon to use the new version.",
            tag
        )
        .bright_green()
        .bold()
    );
    Ok(())
}

fn platform_asset_name() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("devimon-macos-arm64"),
        ("macos", "x86_64") => Ok("devimon-macos-x86_64"),
        ("linux", "x86_64") => Ok("devimon-linux-x86_64"),
        ("linux", "aarch64") => Ok("devimon-linux-arm64"),
        (os, arch) => Err(format!(
            "no pre-built binary for {}-{}. \
             Build from source: cargo install --git https://github.com/{} --locked --force",
            os, arch, GITHUB_REPO
        )),
    }
}

fn cmd_sync() -> Result<(), String> {
    let mut state = load_state_or_err()?;
    state.cloud.sync_dirty = true;
    let sync = cloud::sync_state(&mut state)?;
    save::save_state(&state).map_err(|e| e.to_string())?;
    print_sync_status(&sync);
    Ok(())
}
