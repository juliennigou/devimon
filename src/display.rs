use crate::monster::{Monster, Species, Stage};
use colored::*;

#[derive(Clone, Copy)]
pub struct AnimationContext {
    pub tick: u64,
    pub area_width: u16,
    pub area_height: u16,
    pub reserve_right: u16,
}

pub struct SpriteScene {
    pub lines: Vec<String>,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy)]
pub enum GameSpritePose {
    Waiting,
    RunA,
    RunB,
    Jump,
    Fall,
    DuckA,
    DuckB,
    Crashed,
}

struct SpeciesRenderer {
    small_art: fn(&Monster) -> Vec<String>,
    scene: fn(&Monster, AnimationContext) -> SpriteScene,
    game_sprite: fn(&Monster, GameSpritePose) -> Vec<String>,
}

const DEVIMON_RENDERER: SpeciesRenderer = SpeciesRenderer {
    small_art: devimon_small_art,
    scene: devimon_scene,
    game_sprite: devimon_game_sprite,
};

const DRAGON_RENDERER: SpeciesRenderer = SpeciesRenderer {
    small_art: dragon_small_art,
    scene: dragon_scene,
    game_sprite: dragon_game_sprite,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnimationState {
    Idle,
    Walk,
    Fly,
}

fn renderer(species: Species) -> &'static SpeciesRenderer {
    match species {
        Species::Devimon => &DEVIMON_RENDERER,
        Species::Dragon => &DRAGON_RENDERER,
    }
}

/// Compact status art used by the CLI `status` command.
pub fn ascii_art(monster: &Monster) -> Vec<String> {
    (renderer(monster.species).small_art)(monster)
}

/// Produce a species-specific animated scene for the TUI home panel.
pub fn tui_scene(
    monster: &Monster,
    tick: u64,
    area_width: u16,
    area_height: u16,
    reserve_right: u16,
) -> SpriteScene {
    (renderer(monster.species).scene)(
        monster,
        AnimationContext {
            tick,
            area_width,
            area_height,
            reserve_right,
        },
    )
}

/// Fixed-footprint runner sprite for side-scroller mini games.
pub fn game_runner_sprite(monster: &Monster, pose: GameSpritePose) -> Vec<String> {
    (renderer(monster.species).game_sprite)(monster, pose)
}

fn devimon_small_art(monster: &Monster) -> Vec<String> {
    let face = devimon_face(monster, 0, false);
    match monster.stage {
        Stage::Baby => vec![
            "   .-^-.".to_string(),
            format!(" .( {} ).", face),
            "  /|___|\\".to_string(),
            "  d_/ \\_b".to_string(),
        ],
        Stage::Young => vec![
            "   /\\_/\\ ".to_string(),
            format!(" .( {} ).", face),
            " /_|___|_\\".to_string(),
            " d_/   \\_b".to_string(),
        ],
        Stage::Evolved => vec![
            "   __/\\\\__".to_string(),
            format!(" .<( {} )>.", face),
            " /_|_____|_\\".to_string(),
            " d_/     \\_b".to_string(),
        ],
    }
}

fn devimon_scene(monster: &Monster, ctx: AnimationContext) -> SpriteScene {
    let state = match classify_mood(monster) {
        MoodState::Tired | MoodState::Hungry | MoodState::Sad => AnimationState::Idle,
        MoodState::Proud | MoodState::Fine => AnimationState::Walk,
    };

    let lines = match monster.stage {
        Stage::Baby => match state {
            AnimationState::Idle => devimon_baby_idle(monster, ctx.tick),
            _ => devimon_baby_walk(monster, ctx.tick),
        },
        Stage::Young => match state {
            AnimationState::Idle => devimon_young_idle(monster, ctx.tick),
            _ => devimon_young_walk(monster, ctx.tick),
        },
        Stage::Evolved => match state {
            AnimationState::Idle => devimon_evolved_idle(monster, ctx.tick),
            _ => devimon_evolved_walk(monster, ctx.tick),
        },
    };

    let (sprite_w, sprite_h) = sprite_size(&lines);
    let (x, y) = match state {
        AnimationState::Idle => idle_motion(ctx, sprite_w, sprite_h),
        AnimationState::Walk => wander_motion(ctx, sprite_w, sprite_h, 0.13, 0.09),
        AnimationState::Fly => wander_motion(ctx, sprite_w, sprite_h, 0.17, 0.12),
    };

    SpriteScene { lines, x, y }
}

fn devimon_baby_idle(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, false);
    let puff = if (tick / 5).is_multiple_of(2) {
        "      .-^-."
    } else {
        "     .-^-."
    };
    vec![
        puff.to_string(),
        format!("   .-( {} )-.", face),
        "  /  / ___ \\  \\".to_string(),
        " |  | / _ \\ |  |".to_string(),
        " |  | \\___/ |  |".to_string(),
        "  \\  \\_____/  /".to_string(),
        "   /_/|   |\\_\\".to_string(),
        "   d_/     \\_b".to_string(),
    ]
}

fn devimon_baby_walk(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, false);
    match (tick / 2) % 4 {
        1 => vec![
            "      .-^-.".to_string(),
            format!("   .-( {} )-.", face),
            "  /  / ___ \\  \\".to_string(),
            " |  | / _ \\ |  |".to_string(),
            " |  | \\___/ |  |".to_string(),
            "  \\  \\_____/  /".to_string(),
            "   /_/|   |\\_\\".to_string(),
            "    b_\\   /_d ".to_string(),
        ],
        3 => vec![
            "      .-^-.".to_string(),
            format!("   .-( {} )-.", face),
            "  /  / ___ \\  \\".to_string(),
            " |  | / _ \\ |  |".to_string(),
            " |  | \\___/ |  |".to_string(),
            "  \\  \\_____/  /".to_string(),
            "   /_/|   |\\_\\".to_string(),
            "   d_/   \\_b  ".to_string(),
        ],
        _ => devimon_baby_idle(monster, tick),
    }
}

fn devimon_young_idle(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, false);
    vec![
        "       /\\_/\\ ".to_string(),
        format!("   .--( {} )--.", face),
        "  /  /| ___ |\\  \\".to_string(),
        " /  / |/ _ \\| \\  \\".to_string(),
        "|  |  | |_| |  |  |".to_string(),
        "|  |  |_____|  |  |".to_string(),
        " \\  \\___/ \\___/  /".to_string(),
        "  d_/       \\_b ".to_string(),
    ]
}

fn devimon_young_walk(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, false);
    match (tick / 2) % 4 {
        1 => vec![
            "       /\\_/\\ ".to_string(),
            format!("   .--( {} )--.", face),
            "  /  /| ___ |\\  \\".to_string(),
            " /  / |/ _ \\| \\  \\".to_string(),
            "|  |  | |_| |  |  |".to_string(),
            "|  |  |_____|  |  |".to_string(),
            " \\  \\___/ \\___/  /".to_string(),
            "   b_\\       /_d ".to_string(),
        ],
        3 => vec![
            "       /\\_/\\ ".to_string(),
            format!("   .--( {} )--.", face),
            "  /  /| ___ |\\  \\".to_string(),
            " /  / |/ _ \\| \\  \\".to_string(),
            "|  |  | |_| |  |  |".to_string(),
            "|  |  |_____|  |  |".to_string(),
            " \\  \\___/ \\___/  /".to_string(),
            "  d_/       \\_b  ".to_string(),
        ],
        _ => devimon_young_idle(monster, tick),
    }
}

fn devimon_evolved_idle(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, true);
    vec![
        "       __/\\\\__".to_string(),
        format!("   .-<( {} )>-.", face),
        "  /  /|  _  |\\  \\".to_string(),
        " /  / | /_\\ | \\  \\".to_string(),
        "|  |  | |_| |  |  |".to_string(),
        "|  |  |_____|  |  |".to_string(),
        " \\  \\__\\___/__/  /".to_string(),
        "  \\____/   \\____/".to_string(),
        "   d_/       \\_b ".to_string(),
    ]
}

fn devimon_evolved_walk(monster: &Monster, tick: u64) -> Vec<String> {
    let face = devimon_face(monster, tick, true);
    match (tick / 2) % 4 {
        1 => vec![
            "       __/\\\\__".to_string(),
            format!("   .-<( {} )>-.", face),
            "  /  /|  _  |\\  \\".to_string(),
            " /  / | /_\\ | \\  \\".to_string(),
            "|  |  | |_| |  |  |".to_string(),
            "|  |  |_____|  |  |".to_string(),
            " \\  \\__\\___/__/  /".to_string(),
            "  \\____/   \\____/".to_string(),
            "    b_\\     /_d  ".to_string(),
        ],
        3 => vec![
            "       __/\\\\__".to_string(),
            format!("   .-<( {} )>-.", face),
            "  /  /|  _  |\\  \\".to_string(),
            " /  / | /_\\ | \\  \\".to_string(),
            "|  |  | |_| |  |  |".to_string(),
            "|  |  |_____|  |  |".to_string(),
            " \\  \\__\\___/__/  /".to_string(),
            "  \\____/   \\____/".to_string(),
            "   d_/     \\_b   ".to_string(),
        ],
        _ => devimon_evolved_idle(monster, tick),
    }
}

fn dragon_small_art(_monster: &Monster) -> Vec<String> {
    vec![
        "   /\\_/\\\\ ".to_string(),
        "  ( `-' )".to_string(),
        "  /|_ _|\\".to_string(),
        "   c   b ".to_string(),
    ]
}

fn dragon_scene(monster: &Monster, ctx: AnimationContext) -> SpriteScene {
    let state = match classify_mood(monster) {
        MoodState::Tired | MoodState::Hungry | MoodState::Sad => AnimationState::Idle,
        MoodState::Proud | MoodState::Fine => AnimationState::Fly,
    };

    let lines = match state {
        AnimationState::Fly => dragon_flight_sprite(ctx.tick),
        _ => dragon_perch_sprite(ctx.tick),
    };
    let (sprite_w, sprite_h) = sprite_size(&lines);
    let (x, y) = match state {
        AnimationState::Idle => idle_motion(ctx, sprite_w, sprite_h),
        AnimationState::Fly => wander_motion(ctx, sprite_w, sprite_h, 0.20, 0.13),
        AnimationState::Walk => wander_motion(ctx, sprite_w, sprite_h, 0.16, 0.10),
    };

    let _ = monster;
    SpriteScene { lines, x, y }
}

fn dragon_perch_sprite(tick: u64) -> Vec<String> {
    let eyes = if is_blinking(tick, 19) { "-.-" } else { "`-'" };
    let belly = if (tick / 5).is_multiple_of(2) {
        "   /|_ _|\\"
    } else {
        "   /|_-_|\\"
    };
    vec![
        "     /\\_/\\\\ ".to_string(),
        format!("   .( {} ).", eyes),
        belly.to_string(),
        "    /  ^  \\".to_string(),
        "    c_/ \\_b".to_string(),
    ]
}

fn dragon_flight_sprite(tick: u64) -> Vec<String> {
    let wings_up = (tick / 2).is_multiple_of(2);
    if wings_up {
        vec![
            r"        __/\__".to_string(),
            r"   .--<( `-' )>--.".to_string(),
            r"      /|/^\|\ ".to_string(),
            r"     c_/   \_b".to_string(),
        ]
    } else {
        vec![
            r"      __/\/\__".to_string(),
            r"  .--<( `-' )>--.".to_string(),
            r"      /|/^\|\ ".to_string(),
            r"     c_/   \_b".to_string(),
        ]
    }
}

fn devimon_game_sprite(monster: &Monster, pose: GameSpritePose) -> Vec<String> {
    let face = match monster.stage {
        Stage::Evolved => devimon_face(monster, 0, true),
        _ => devimon_face(monster, 0, false),
    };

    let lines = match pose {
        GameSpritePose::Waiting => vec![
            "   /^^\\   ".to_string(),
            format!("  ( {} )  ", face),
            "  /|___|\\ ".to_string(),
            "   / | \\  ".to_string(),
            "  d_/ \\_b ".to_string(),
        ],
        GameSpritePose::Jump => vec![
            "   /^^\\   ".to_string(),
            format!("  ( {} )  ", face),
            "  /|___|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  _/   \\_ ".to_string(),
        ],
        GameSpritePose::Fall => vec![
            "   /^^\\   ".to_string(),
            format!("  ( {} )  ", face),
            "  /|___|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  d_   _b ".to_string(),
        ],
        GameSpritePose::DuckA => vec![
            "          ".to_string(),
            format!("  ( {} )__ ", face),
            " _/|___ __\\".to_string(),
            " d_______b ".to_string(),
            "          ".to_string(),
        ],
        GameSpritePose::DuckB => vec![
            "          ".to_string(),
            format!(" __( {} )  ", face),
            "/__ ___|\\_ ".to_string(),
            " b_______d ".to_string(),
            "          ".to_string(),
        ],
        GameSpritePose::Crashed => vec![
            "   /^^\\   ".to_string(),
            "  ( x_x ) ".to_string(),
            "  /|___|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  d_   _b ".to_string(),
        ],
        GameSpritePose::RunB => vec![
            "   /^^\\   ".to_string(),
            format!("  ( {} )  ", face),
            "  /|___|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  d_/ \\_b ".to_string(),
        ],
        GameSpritePose::RunA => vec![
            "   /^^\\   ".to_string(),
            format!("  ( {} )  ", face),
            "  /|___|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  b_/ \\_d ".to_string(),
        ],
    };
    pad_sprite(lines, 10, 5)
}

fn dragon_game_sprite(_monster: &Monster, pose: GameSpritePose) -> Vec<String> {
    let lines = match pose {
        GameSpritePose::Waiting => vec![
            "   /\\_/\\  ".to_string(),
            "  ( `-' ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   / | \\  ".to_string(),
            "  c_/ \\_b ".to_string(),
        ],
        GameSpritePose::Jump => vec![
            "   /\\_/\\  ".to_string(),
            "  ( `-' ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  _c   b_ ".to_string(),
        ],
        GameSpritePose::Fall => vec![
            "   /\\_/\\  ".to_string(),
            "  ( `-' ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  c_   _b ".to_string(),
        ],
        GameSpritePose::DuckA => vec![
            "          ".to_string(),
            "  ( `-' )_".to_string(),
            " _/|_^|__\\".to_string(),
            " c_______b".to_string(),
            "          ".to_string(),
        ],
        GameSpritePose::DuckB => vec![
            "          ".to_string(),
            "_( `-' )  ".to_string(),
            "/__|^_|\\_ ".to_string(),
            " b_______c".to_string(),
            "          ".to_string(),
        ],
        GameSpritePose::Crashed => vec![
            "   /\\_/\\  ".to_string(),
            "  ( x_x ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  c_   _b ".to_string(),
        ],
        GameSpritePose::RunB => vec![
            "   /\\_/\\  ".to_string(),
            "  ( `-' ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  c_/ \\_b ".to_string(),
        ],
        GameSpritePose::RunA => vec![
            "   /\\_/\\  ".to_string(),
            "  ( `-' ) ".to_string(),
            "  /|_^_|\\ ".to_string(),
            "   /   \\  ".to_string(),
            "  b_/ \\_c ".to_string(),
        ],
    };
    pad_sprite(lines, 10, 5)
}

fn devimon_face(monster: &Monster, tick: u64, fierce: bool) -> &'static str {
    if is_blinking(tick, 17) {
        return "-.-";
    }
    if fierce {
        if monster.mood >= 70.0 {
            "O_O"
        } else if monster.mood >= 35.0 {
            "o_o"
        } else {
            "x_x"
        }
    } else if monster.mood >= 70.0 {
        "^o^"
    } else if monster.mood >= 35.0 {
        "-_-"
    } else {
        ";_;"
    }
}

fn is_blinking(tick: u64, period: u64) -> bool {
    tick % period == period - 1
}

fn sprite_size(lines: &[String]) -> (u16, u16) {
    let width = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u16;
    let height = lines.len() as u16;
    (width, height)
}

fn pad_sprite(mut lines: Vec<String>, width: usize, height: usize) -> Vec<String> {
    for line in &mut lines {
        let len = line.chars().count();
        if len < width {
            line.push_str(&" ".repeat(width - len));
        }
    }
    while lines.len() < height {
        lines.push(" ".repeat(width));
    }
    lines
}

fn travel_width(ctx: AnimationContext, sprite_w: u16) -> u16 {
    let usable = if ctx.area_width > ctx.reserve_right {
        ctx.area_width.saturating_sub(ctx.reserve_right)
    } else {
        ctx.area_width
    };
    usable.max(sprite_w).min(ctx.area_width)
}

fn idle_motion(ctx: AnimationContext, sprite_w: u16, sprite_h: u16) -> (u16, u16) {
    let travel_w = travel_width(ctx, sprite_w);
    let max_x = travel_w.saturating_sub(sprite_w);
    let max_y = ctx.area_height.saturating_sub(sprite_h);
    let t = ctx.tick as f64;

    let base_x = max_x as f64 / 2.0;
    let base_y = max_y as f64 / 2.0;
    let sway = (t * 0.09).sin() * f64::from(max_x.min(3)) * 0.5;
    let bob = (t * 0.18).sin() * f64::from(max_y.min(2)) * 0.5;

    (
        (base_x + sway).round().clamp(0.0, f64::from(max_x)) as u16,
        (base_y + bob).round().clamp(0.0, f64::from(max_y)) as u16,
    )
}

fn wander_motion(
    ctx: AnimationContext,
    sprite_w: u16,
    sprite_h: u16,
    x_speed: f64,
    y_speed: f64,
) -> (u16, u16) {
    let travel_w = travel_width(ctx, sprite_w);
    let max_x = travel_w.saturating_sub(sprite_w) as f64;
    let max_y = ctx.area_height.saturating_sub(sprite_h) as f64;
    let t = ctx.tick as f64;

    let norm_x = (t * x_speed).sin() * 0.5 + 0.5;
    let norm_y = (t * y_speed).cos() * 0.5 + 0.5;

    (
        (norm_x * max_x).round() as u16,
        (norm_y * max_y).round() as u16,
    )
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
        MoodState::Tired => format!("{} semble epuise...", monster.name),
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
