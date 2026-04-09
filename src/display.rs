use crate::monster::{Monster, Stage};
use colored::*;

/// Small 3-line art used by the CLI status command.
pub fn ascii_art(monster: &Monster) -> Vec<String> {
    let face = mood_face_small(monster.mood);
    match monster.stage {
        Stage::Baby => vec![
            format!("   {}   ", face),
            "   /||\\  ".to_string(),
            "   d  b  ".to_string(),
        ],
        Stage::Young => vec![
            format!("  {}   ", face),
            "  /|||\\ ".to_string(),
            "  d   b ".to_string(),
        ],
        Stage::Evolved => vec![
            format!(" {}   ", face),
            " /||||\\".to_string(),
            " d    b".to_string(),
        ],
    }
}

/// Larger 5-line art used by the TUI.
pub fn ascii_art_big(monster: &Monster) -> Vec<String> {
    match monster.stage {
        Stage::Baby => {
            let face = mood_face_small(monster.mood);
            vec![
                format!("    ( {} )", face),
                "   (       )".to_string(),
                "    \\_____/".to_string(),
                "     /|||\\ ".to_string(),
                "    d     b ".to_string(),
            ]
        }
        Stage::Young => {
            let face = mood_face_small(monster.mood);
            vec![
                format!("   \\( {} )/", face),
                "   (         )".to_string(),
                "    \\_______/".to_string(),
                "     /|||||\\  ".to_string(),
                "   d         b".to_string(),
            ]
        }
        Stage::Evolved => {
            let face = mood_face_evolved(monster.mood);
            vec![
                format!("  \\\\( {} )//", face),
                "   (           )".to_string(),
                "    \\_________/".to_string(),
                "     /|||||||\\  ".to_string(),
                "   d           b".to_string(),
            ]
        }
    }
}

fn mood_face_small(mood: f32) -> &'static str {
    if mood >= 60.0 {
        "^o^"
    } else if mood >= 30.0 {
        "-_-"
    } else {
        ";_;"
    }
}

fn mood_face_evolved(mood: f32) -> &'static str {
    if mood >= 60.0 {
        ">O<"
    } else if mood >= 30.0 {
        "•_•"
    } else {
        "T_T"
    }
}

/// Render a 20-cell bar with value / 100 fill.
pub fn bar(value: f32, label: &str) -> String {
    let width = 20;
    let filled = ((value / 100.0) * width as f32).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let fill_str: String = "█".repeat(filled);
    let empty_str: String = "░".repeat(empty);

    let colored_fill = if value >= 60.0 {
        fill_str.green()
    } else if value >= 30.0 {
        fill_str.yellow()
    } else {
        fill_str.red()
    };

    format!(
        "{:<8} {}{} {:>3.0}/100",
        label,
        colored_fill,
        empty_str.bright_black(),
        value
    )
}

#[derive(Debug, Clone, Copy)]
pub enum MoodState {
    Tired,
    Hungry,
    Sad,
    Proud,
    Fine,
}

pub fn classify_mood(monster: &Monster) -> MoodState {
    if monster.energy < 20.0 {
        MoodState::Tired
    } else if monster.hunger < 20.0 {
        MoodState::Hungry
    } else if monster.mood < 30.0 {
        MoodState::Sad
    } else if monster.mood > 80.0 && monster.energy > 60.0 {
        MoodState::Proud
    } else {
        MoodState::Fine
    }
}

/// Plain-text personality message (for the TUI, which handles styling itself).
pub fn personality_text(monster: &Monster) -> String {
    match classify_mood(monster) {
        MoodState::Tired => format!("{} semble épuisé…", monster.name),
        MoodState::Hungry => format!("{} a faim.", monster.name),
        MoodState::Sad => format!("{} est triste.", monster.name),
        MoodState::Proud => format!("{} est fier de toi !", monster.name),
        MoodState::Fine => format!("{} va bien.", monster.name),
    }
}

/// ANSI-colored personality line for the CLI `status` command.
pub fn personality_line(monster: &Monster) -> String {
    let msg = personality_text(monster);
    match classify_mood(monster) {
        MoodState::Tired => format!("{}", msg.bright_black()),
        MoodState::Hungry => format!("{}", msg.yellow()),
        MoodState::Sad => format!("{}", msg.red()),
        MoodState::Proud => format!("{}", msg.bright_green()),
        MoodState::Fine => format!("{}", msg.cyan()),
    }
}

pub fn render_status(monster: &Monster, xp_gained: u32) {
    let art = ascii_art(monster);
    println!();
    for line in &art {
        println!("  {}", line.bright_magenta());
    }
    println!();
    println!(
        "  {} — {} {} {}",
        monster.name.bold(),
        format!("lv.{}", monster.level).bright_yellow(),
        "·".bright_black(),
        monster.stage.label().bright_blue()
    );
    println!(
        "  {}",
        format!("XP: {}/{}", monster.xp, monster.xp_to_next()).bright_black()
    );
    println!();
    println!("  {}", bar(monster.hunger, "Faim"));
    println!("  {}", bar(monster.energy, "Énergie"));
    println!("  {}", bar(monster.mood, "Moral"));
    println!();
    println!("  {}", personality_line(monster));
    if xp_gained > 0 {
        println!(
            "  {}",
            format!("(+{} XP depuis la dernière visite)", xp_gained).bright_green()
        );
    }
    println!();
}
