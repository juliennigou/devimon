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
                    if should_ignore(&p) {
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

fn should_ignore(path: &Path) -> bool {
    let ignored_dirs = [
        ".git",
        "target",
        "node_modules",
        ".devimon",
        "dist",
        "build",
        ".next",
        ".cache",
    ];
    let path_str = path.as_os_str().to_string_lossy();
    let mut segments = path_str
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty());

    if segments
        .clone()
        .any(|segment| ignored_dirs.contains(&segment))
    {
        return true;
    }

    // Hidden files and editor swap files.
    if let Some(name) = segments.next_back() {
        if name.starts_with('.') || name.ends_with('~') || name.ends_with(".swp") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::should_ignore;
    use std::path::Path;

    #[test]
    fn ignores_common_directories_with_unix_paths() {
        assert!(should_ignore(Path::new("/tmp/project/.git/config")));
        assert!(should_ignore(Path::new(
            "/tmp/project/node_modules/react/index.js"
        )));
    }

    #[test]
    fn ignores_common_directories_with_windows_paths() {
        assert!(should_ignore(Path::new(
            r"C:\Users\dev\project\.git\config"
        )));
        assert!(should_ignore(Path::new(
            r"C:\Users\dev\project\node_modules\react\index.js"
        )));
    }

    #[test]
    fn ignores_hidden_and_swap_files() {
        assert!(should_ignore(Path::new("/tmp/project/.env")));
        assert!(should_ignore(Path::new("/tmp/project/main.rs.swp")));
    }

    #[test]
    fn keeps_normal_source_files() {
        assert!(!should_ignore(Path::new("/tmp/project/src/main.rs")));
    }
}
