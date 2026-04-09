use crate::xp::{XpEvent, append_event};
use chrono::Utc;
use notify::{EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

/// Watch the given directory and append XP events for every file modification.
/// Ignores dotfiles and common build directories so random tooling noise
/// doesn't inflate XP.
pub fn watch(path: &Path) -> notify::Result<()> {
    println!("👀 Watching {} — press Ctrl+C to stop.", path.display());
    watch_inner(path)
}

/// Same as [`watch`] but without the stdout banner — used when the watcher
/// runs inside the TUI as a background thread.
pub fn watch_silent(path: &Path) -> notify::Result<()> {
    watch_inner(path)
}

fn watch_inner(path: &Path) -> notify::Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    let mut last_logged: std::collections::HashMap<String, Instant> =
        std::collections::HashMap::new();
    let debounce = Duration::from_secs(2);

    for res in rx {
        match res {
            Ok(event) => {
                if !matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    continue;
                }
                for p in event.paths {
                    let path_str = p.to_string_lossy().to_string();
                    if should_ignore(&path_str) {
                        continue;
                    }
                    // Debounce repeated writes to the same file.
                    let now = Instant::now();
                    if let Some(t) = last_logged.get(&path_str) {
                        if now.duration_since(*t) < debounce {
                            continue;
                        }
                    }
                    last_logged.insert(path_str.clone(), now);

                    let ev = XpEvent {
                        kind: "file_modified".to_string(),
                        path: path_str,
                        timestamp: Utc::now(),
                    };
                    if let Err(e) = append_event(&ev) {
                        eprintln!("warn: failed to append event: {}", e);
                    }
                }
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }
    Ok(())
}

fn should_ignore(path: &str) -> bool {
    let ignored_segments = [
        "/.git/",
        "/target/",
        "/node_modules/",
        "/.devimon/",
        "/dist/",
        "/build/",
        "/.next/",
        "/.cache/",
    ];
    if ignored_segments.iter().any(|seg| path.contains(seg)) {
        return true;
    }
    // Hidden files and editor swap files.
    if let Some(name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.ends_with('~') || name.ends_with(".swp") {
            return true;
        }
    }
    false
}
