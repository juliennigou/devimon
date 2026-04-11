use crate::monster::{Monster, Species, Stage};
use colored::*;

// ── Public types ──────────────────────────────────────────────────────────────

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

// ── Form data model ───────────────────────────────────────────────────────────
//
// A `MonsterForm` is a fully data-driven description of one species+stage.
// Adding a new monster = adding a new const `MonsterForm` and wiring it in
// `form()`.  No new functions, no per-species dispatch tables.
//
// Frame strings may contain a `{f}` token, which is replaced at render-time
// with a 3-character mood-reactive face (e.g. `^_^`, `o_o`, `;_;`).

struct MonsterForm {
    /// Compact art for `devimon status` (≤ ~6 rows).
    small: &'static [&'static str],
    /// Animation frames played when the monster is idle/tired.
    idle: &'static [&'static [&'static str]],
    /// Animation frames played when the monster is happy/active.
    active: &'static [&'static [&'static str]],
    /// Per-pose sprites used by the dino runner mini game.
    game: GameSpriteSet,
    /// Idle motion intensity (sway/bob).
    idle_motion: MotionStyle,
    /// Active motion intensity (wander).
    active_motion: MotionStyle,
}

struct GameSpriteSet {
    waiting: &'static [&'static str],
    run_a: &'static [&'static str],
    run_b: &'static [&'static str],
    jump: &'static [&'static str],
    fall: &'static [&'static str],
    duck_a: &'static [&'static str],
    duck_b: &'static [&'static str],
    crashed: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct MotionStyle {
    x_speed: f64,
    y_speed: f64,
}

const IDLE_MOTION: MotionStyle = MotionStyle { x_speed: 0.0, y_speed: 0.0 };
const GENTLE_MOTION: MotionStyle = MotionStyle { x_speed: 0.11, y_speed: 0.07 };
const WALKING_MOTION: MotionStyle = MotionStyle { x_speed: 0.13, y_speed: 0.09 };
const FLOATING_MOTION: MotionStyle = MotionStyle { x_speed: 0.17, y_speed: 0.13 };
const DRIFTING_MOTION: MotionStyle = MotionStyle { x_speed: 0.10, y_speed: 0.16 };

// ── Form table ────────────────────────────────────────────────────────────────

fn form(species: Species, stage: Stage) -> &'static MonsterForm {
    match (species, stage) {
        (Species::Ember, Stage::Baby) => &EMBIT,
        (Species::Ember, Stage::Young) => &PYROFANG,
        (Species::Ember, Stage::Evolved) => &INFERNOX,
        (Species::Tide, Stage::Baby) => &DRIPLET,
        (Species::Tide, Stage::Young) => &WAVEKIN,
        (Species::Tide, Stage::Evolved) => &MAELSTRYX,
        (Species::Bloom, Stage::Baby) => &SPROUT,
        (Species::Bloom, Stage::Young) => &VINEKITH,
        (Species::Bloom, Stage::Evolved) => &ELDROAK,
    }
}

// ── Public render API ─────────────────────────────────────────────────────────

pub fn ascii_art(monster: &Monster) -> Vec<String> {
    let face = face_for(monster);
    form(monster.species, monster.stage)
        .small
        .iter()
        .map(|line| line.replace("{f}", face))
        .collect()
}

pub fn tui_scene(
    monster: &Monster,
    tick: u64,
    area_width: u16,
    area_height: u16,
    reserve_right: u16,
) -> SpriteScene {
    let form = form(monster.species, monster.stage);
    let active = matches!(
        classify_mood(monster),
        MoodState::Proud | MoodState::Fine
    );
    let frames = if active { form.active } else { form.idle };
    let face = face_for(monster);
    let frame_index = (tick / 6) as usize % frames.len().max(1);
    let lines: Vec<String> = frames[frame_index]
        .iter()
        .map(|l| l.replace("{f}", face))
        .collect();

    let (sprite_w, sprite_h) = sprite_size(&lines);
    let ctx = AnimationContext {
        tick,
        area_width,
        area_height,
        reserve_right,
    };
    let motion = if active {
        form.active_motion
    } else {
        form.idle_motion
    };
    let (x, y) = position(ctx, sprite_w, sprite_h, motion);

    SpriteScene { lines, x, y }
}

pub fn game_runner_sprite(monster: &Monster, pose: GameSpritePose) -> Vec<String> {
    let form = form(monster.species, monster.stage);
    let face = face_for(monster);
    let frame: &[&str] = match pose {
        GameSpritePose::Waiting => form.game.waiting,
        GameSpritePose::RunA => form.game.run_a,
        GameSpritePose::RunB => form.game.run_b,
        GameSpritePose::Jump => form.game.jump,
        GameSpritePose::Fall => form.game.fall,
        GameSpritePose::DuckA => form.game.duck_a,
        GameSpritePose::DuckB => form.game.duck_b,
        GameSpritePose::Crashed => form.game.crashed,
    };
    let lines: Vec<String> = frame
        .iter()
        .map(|l| {
            let face = if matches!(pose, GameSpritePose::Crashed) {
                "x_x"
            } else {
                face
            };
            l.replace("{f}", face)
        })
        .collect();
    pad_sprite(lines, 11, 5)
}

// ── Faces ─────────────────────────────────────────────────────────────────────

fn face_for(monster: &Monster) -> &'static str {
    if monster.mood >= 70.0 {
        "^_^"
    } else if monster.mood >= 35.0 {
        "o_o"
    } else {
        ";_;"
    }
}

// ── Geometry & motion ─────────────────────────────────────────────────────────

fn sprite_size(lines: &[String]) -> (u16, u16) {
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
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

fn position(
    ctx: AnimationContext,
    sprite_w: u16,
    sprite_h: u16,
    motion: MotionStyle,
) -> (u16, u16) {
    let travel_w = travel_width(ctx, sprite_w);
    let max_x = travel_w.saturating_sub(sprite_w);
    let max_y = ctx.area_height.saturating_sub(sprite_h);
    let t = ctx.tick as f64;

    if motion.x_speed == 0.0 && motion.y_speed == 0.0 {
        // Pure idle: small sway around the centre.
        let base_x = max_x as f64 / 2.0;
        let base_y = max_y as f64 / 2.0;
        let sway = (t * 0.09).sin() * f64::from(max_x.min(3)) * 0.5;
        let bob = (t * 0.18).sin() * f64::from(max_y.min(2)) * 0.5;
        return (
            (base_x + sway).round().clamp(0.0, f64::from(max_x)) as u16,
            (base_y + bob).round().clamp(0.0, f64::from(max_y)) as u16,
        );
    }

    let norm_x = (t * motion.x_speed).sin() * 0.5 + 0.5;
    let norm_y = (t * motion.y_speed).cos() * 0.5 + 0.5;
    (
        (norm_x * max_x as f64).round() as u16,
        (norm_y * max_y as f64).round() as u16,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// MONSTER FORMS
// ─────────────────────────────────────────────────────────────────────────────
//
// Visual conventions:
//   • `{f}` = 3-character face slot
//   • elemental motifs:
//       - Ember (fire):  `,*,`  `^^^`  `\v/`  flame curls
//       - Tide  (water): `~~~`  `,~,`  `(())` waves & droplets
//       - Bloom (grass): `\|/`  `///`  `\\\`  leaves & vines
//   • size scales with stage: Baby ~5 rows, Young ~7, Evolved ~9-10

// ── EMBER LINE (Fire) ────────────────────────────────────────────────────────

const EMBIT: MonsterForm = MonsterForm {
    small: &[
        r"     ,*,    ",
        r"    /'^'\   ",
        r"   ( {f} )  ",
        r"    \v_v/   ",
        r"     `^`    ",
    ],
    idle: &[
        &[
            r"     ,*,    ",
            r"    /'^'\   ",
            r"   ( {f} )  ",
            r"    \v_v/   ",
            r"     `^`    ",
        ],
        &[
            r"     .*.    ",
            r"    /,^,\   ",
            r"   ( {f} )  ",
            r"    \v_v/   ",
            r"     `^`    ",
        ],
    ],
    active: &[
        &[
            r"     ,*,    ",
            r"    /'^'\   ",
            r"  d-( {f} ) ",
            r"    \v_v/   ",
            r"    ^   ^   ",
        ],
        &[
            r"     ,*,    ",
            r"    /'^'\   ",
            r"   ( {f} )-b",
            r"    \v_v/   ",
            r"    ^   ^   ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: GENTLE_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"   ,*,   ",
            r"  /'^'\  ",
            r" ( {f} ) ",
            r"  \v_v/  ",
            r"  /   \  ",
        ],
        run_a: &[
            r"   ,*,   ",
            r"  /'^'\  ",
            r" ( {f} ) ",
            r"  \v_v/  ",
            r"  d_/\b  ",
        ],
        run_b: &[
            r"   ,*,   ",
            r"  /'^'\  ",
            r" ( {f} ) ",
            r"  \v_v/  ",
            r"  b_/\d  ",
        ],
        jump: &[
            r"   ,*,   ",
            r"  /'^'\  ",
            r" ( {f} ) ",
            r"  \v_v/  ",
            r"  _ _ _  ",
        ],
        fall: &[
            r"   ,*,   ",
            r"  /'^'\  ",
            r" ( {f} ) ",
            r"  \v_v/  ",
            r"  d   b  ",
        ],
        duck_a: &[
            r"         ",
            r"   ,*,   ",
            r"  /( {f} )",
            r" b\v_v/d ",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"   ,*,   ",
            r" ( {f} )\",
            r" d\v_v/b ",
            r"         ",
        ],
        crashed: &[
            r"   ,x,   ",
            r"  /'-'\  ",
            r" ( {f} ) ",
            r"  \_x_/  ",
            r"  d   b  ",
        ],
    },
};

const PYROFANG: MonsterForm = MonsterForm {
    small: &[
        r"   /\,'\,    ",
        r"  //'^'\\    ",
        r" ( ( {f} ) ) ",
        r"  \\v_v//    ",
        r"  d/   \b    ",
    ],
    idle: &[
        &[
            r"    /\,'\,     ",
            r"   //'^'\\     ",
            r"  ( ( {f} ) )  ",
            r"   \\v_v//     ",
            r"   /|/^\|\     ",
            r"   d/   \b     ",
        ],
        &[
            r"    /\,'\,     ",
            r"   //'^'\\     ",
            r"  ( ( {f} ) )  ",
            r"   \\v_v//     ",
            r"   /|\^/|\     ",
            r"   d/   \b     ",
        ],
    ],
    active: &[
        &[
            r"    /\,'\,     ",
            r"   //'^'\\     ",
            r"  ( ( {f} ) )  ",
            r"   \\v_v//     ",
            r"   /|/^\|\     ",
            r"   d/     \b   ",
        ],
        &[
            r"    /\,'\,     ",
            r"   //'^'\\     ",
            r"  ( ( {f} ) )  ",
            r"   \\v_v//     ",
            r"   /|\^/|\     ",
            r"     d/  \b    ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: WALKING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"  /\,'\, ",
            r" //'^'\\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d/   \b ",
        ],
        run_a: &[
            r"  /\,'\, ",
            r" //'^'\\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/  \b ",
        ],
        run_b: &[
            r"  /\,'\, ",
            r" //'^'\\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d/  \_b ",
        ],
        jump: &[
            r"  /\,'\, ",
            r" //'^'\\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r"  _   _  ",
        ],
        fall: &[
            r"  /\,'\, ",
            r" //'^'\\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_   _b ",
        ],
        duck_a: &[
            r"         ",
            r" /\,'\,  ",
            r"( ({f}) )",
            r"d\\v_v//b",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"  ,'\,/\ ",
            r"( ({f}) )",
            r"b\\v_v//d",
            r"         ",
        ],
        crashed: &[
            r"  /x ,x  ",
            r" /-'-'-\ ",
            r"( ({f}) )",
            r" \\_x_// ",
            r" d_   _b ",
        ],
    },
};

const INFERNOX: MonsterForm = MonsterForm {
    small: &[
        r"    ,*,_,*,    ",
        r"   /^/^\^\^\   ",
        r"  /( ({f}) )\  ",
        r"   \\v_v//     ",
        r"    \\___//    ",
        r"    d_/ \_b    ",
    ],
    idle: &[
        &[
            r"     ,*,_,*,      ",
            r"    /^/^\^\^\     ",
            r"   /(   _   )\    ",
            r"  / ( ({f}) ) \   ",
            r"   \\\v_v///      ",
            r"    \\\___///     ",
            r"    /|\\_//|\     ",
            r"    d_/   \_b     ",
        ],
        &[
            r"     ,*,_,*,      ",
            r"    /^/^\^\^\     ",
            r"   /(   _   )\    ",
            r"  ( ( ({f}) ) )   ",
            r"   \\\v_v///      ",
            r"    \\\___///     ",
            r"    /|\\_//|\     ",
            r"    d_/   \_b     ",
        ],
    ],
    active: &[
        &[
            r"   ,*,_,*,_,*,    ",
            r"   \^/^\^\^\^/    ",
            r"   /(   _   )\    ",
            r" _( ( ({f}) ) )_  ",
            r"   \\\v_v///      ",
            r"    \\\___///     ",
            r"    /|\\_//|\     ",
            r"    d_/   \_b     ",
        ],
        &[
            r"   ,*, ,*, ,*,    ",
            r"   /^/ ^\^/ ^\    ",
            r"   /(   _   )\    ",
            r"  ( ( ({f}) ) )   ",
            r"   \\\v_v///      ",
            r"    \\\___///     ",
            r"   /||\\_//||\    ",
            r"   d_/     \_b    ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: WALKING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r" ,*,_,*, ",
            r" /^/^\^\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/ \_b ",
        ],
        run_a: &[
            r" ,*,_,*, ",
            r" /^/^\^\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d__/\_b ",
        ],
        run_b: &[
            r" ,*,_,*, ",
            r" /^/^\^\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/\__b ",
        ],
        jump: &[
            r" ,*,_,*, ",
            r" /^/^\^\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r"  _   _  ",
        ],
        fall: &[
            r" ,*,_,*, ",
            r" /^/^\^\ ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_   _b ",
        ],
        duck_a: &[
            r"         ",
            r",*,_,*,_ ",
            r"( ({f}) )",
            r"d\\v_v//b",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"_,*,_,*, ",
            r"( ({f}) )",
            r"b\\v_v//d",
            r"         ",
        ],
        crashed: &[
            r" ,x,_,x, ",
            r" /-/-\-\ ",
            r"( ({f}) )",
            r" \\_x_// ",
            r" d_   _b ",
        ],
    },
};

// ── TIDE LINE (Water) ────────────────────────────────────────────────────────

const DRIPLET: MonsterForm = MonsterForm {
    small: &[
        r"      .      ",
        r"     /'\     ",
        r"    ( {f} )  ",
        r"     \_/     ",
        r"     `~`     ",
    ],
    idle: &[
        &[
            r"      .      ",
            r"     /'\     ",
            r"    ( {f} )  ",
            r"     \_/     ",
            r"     `~`     ",
        ],
        &[
            r"      .      ",
            r"     /,\     ",
            r"    ( {f} )  ",
            r"     \_/     ",
            r"     ~`~     ",
        ],
    ],
    active: &[
        &[
            r"      .      ",
            r"     /'\     ",
            r"  d-( {f} )  ",
            r"     \_/     ",
            r"     `~`     ",
        ],
        &[
            r"      ,      ",
            r"     /,\     ",
            r"    ( {f} )-b",
            r"     \_/     ",
            r"     ~`~     ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: DRIFTING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"    .    ",
            r"   /'\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"   `~`   ",
        ],
        run_a: &[
            r"    .    ",
            r"   /'\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d/\b   ",
        ],
        run_b: &[
            r"    .    ",
            r"   /'\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"   b/\d  ",
        ],
        jump: &[
            r"    .    ",
            r"   /'\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  _   _  ",
        ],
        fall: &[
            r"    .    ",
            r"   /'\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d   b  ",
        ],
        duck_a: &[
            r"         ",
            r"   ,/`,  ",
            r" d( {f} )",
            r"  ~\_/~b ",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"  ,'\,   ",
            r"  ( {f} )b",
            r" d~\_/~  ",
            r"         ",
        ],
        crashed: &[
            r"    x    ",
            r"   /-\   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d   b  ",
        ],
    },
};

const WAVEKIN: MonsterForm = MonsterForm {
    small: &[
        r"    _,~,_     ",
        r"  ,~~   ~~,   ",
        r" ( ( {f} ) )  ",
        r"  \\\v_v///   ",
        r"   ~~~~~~~    ",
    ],
    idle: &[
        &[
            r"    _,~,_      ",
            r"  ,~~   ~~,    ",
            r" ( ( {f} ) )   ",
            r"  \\\v_v///    ",
            r"   ~~~~~~~     ",
            r"   ~~   ~~     ",
        ],
        &[
            r"    _,~,_      ",
            r"   ,~ ~ ~,     ",
            r" ( ( {f} ) )   ",
            r"  \\\v_v///    ",
            r"   ~~~~~~~     ",
            r"    ~~~~~      ",
        ],
    ],
    active: &[
        &[
            r"   _,~,_,~,_    ",
            r"  ,~~ ~ ~~,     ",
            r" ( ( {f} ) )    ",
            r"  \\\v_v///     ",
            r"   ~~~~~~~      ",
            r"   ~~     ~~    ",
        ],
        &[
            r"   _,~~,_,~,    ",
            r"  ,~ ~~ ~~,     ",
            r" ( ( {f} ) )    ",
            r"  \\\v_v///     ",
            r"   ~~~~~~~      ",
            r"     ~~~        ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: FLOATING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"  _,~,_  ",
            r" ,~~ ~~, ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~~~ ~~~ ",
        ],
        run_a: &[
            r"  _,~,_  ",
            r" ,~~ ~~, ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~~/ \~~ ",
        ],
        run_b: &[
            r"  _,~,_  ",
            r" ,~~ ~~, ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~\   /~ ",
        ],
        jump: &[
            r"  _,~,_  ",
            r" ,~~ ~~, ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~~   ~~ ",
        ],
        fall: &[
            r"  _,~,_  ",
            r" ,~~ ~~, ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~     ~ ",
        ],
        duck_a: &[
            r"         ",
            r" _,~,_,~,",
            r"( ({f}) )",
            r"~\\v_v//~",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r",~,_,~,_ ",
            r"( ({f}) )",
            r"~\\v_v//~",
            r"         ",
        ],
        crashed: &[
            r"  _,x,_  ",
            r" ,~ - ~, ",
            r"( ({f}) )",
            r" \\_x_// ",
            r" ~     ~ ",
        ],
    },
};

const MAELSTRYX: MonsterForm = MonsterForm {
    small: &[
        r"    _,~~~~~,_     ",
        r"   ,~ ~~~~~ ~,    ",
        r"  (( ({f}) ))     ",
        r"   \\\v_v///      ",
        r"    \\\___///     ",
        r"    ~~~   ~~~     ",
    ],
    idle: &[
        &[
            r"     _,~~~~~,_      ",
            r"    ,~ ~~~~~ ~,     ",
            r"   ((  ___  ))      ",
            r"  (( ( {f} ) ))     ",
            r"   \\\\v_v////      ",
            r"    \\\\___////     ",
            r"     ((((^))))      ",
            r"     ~~~ ~~~~       ",
            r"    ~~~   ~~~       ",
        ],
        &[
            r"     _,~~~~~,_      ",
            r"    ,~ ~~~~~ ~,     ",
            r"   ((  ___  ))      ",
            r"  (( ( {f} ) ))     ",
            r"   \\\\v_v////      ",
            r"    \\\\___////     ",
            r"     ((((^))))      ",
            r"      ~~~~~~        ",
            r"     ~~~ ~~~        ",
        ],
    ],
    active: &[
        &[
            r"   _,~~,_,~~,_      ",
            r"  ,~ ~~ ~ ~~ ~,     ",
            r"   ((  ___  ))      ",
            r" (( ( ({f}) ) ))    ",
            r"   \\\\v_v////      ",
            r"    \\\\___////     ",
            r"     ((((^))))      ",
            r"   ~~~       ~~~    ",
            r"  ~~           ~~   ",
        ],
        &[
            r"    ~,~~~~~~,~      ",
            r"   ,~ ~~~~~~ ~,     ",
            r"   ((  ___  ))      ",
            r"  (( ( {f} ) ))     ",
            r"   \\\\v_v////      ",
            r"    \\\\___////     ",
            r"     ((((^))))      ",
            r"      ~~~~~~~       ",
            r"     ~~~~~~~~~      ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: FLOATING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r" _,~~~,_ ",
            r",~ ~~~ ~,",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~~~ ~~~ ",
        ],
        run_a: &[
            r" _,~~~,_ ",
            r",~ ~~~ ~,",
            r"( ({f}) )",
            r" \\v_v// ",
            r"~~/   \~~",
        ],
        run_b: &[
            r" _,~~~,_ ",
            r",~ ~~~ ~,",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~\   /~ ",
        ],
        jump: &[
            r" _,~~~,_ ",
            r",~ ~~~ ~,",
            r"( ({f}) )",
            r" \\v_v// ",
            r"~~     ~~",
        ],
        fall: &[
            r" _,~~~,_ ",
            r",~ ~~~ ~,",
            r"( ({f}) )",
            r" \\v_v// ",
            r" ~     ~ ",
        ],
        duck_a: &[
            r"         ",
            r"_,~~~,_~,",
            r"( ({f}) )",
            r"~\\v_v//~",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r",~_,~~~,_",
            r"( ({f}) )",
            r"~\\v_v//~",
            r"         ",
        ],
        crashed: &[
            r" _,x x,_ ",
            r",~ - - ~,",
            r"( ({f}) )",
            r" \\_x_// ",
            r" ~     ~ ",
        ],
    },
};

// ── BLOOM LINE (Grass) ───────────────────────────────────────────────────────

const SPROUT: MonsterForm = MonsterForm {
    small: &[
        r"    \\|//    ",
        r"     \|/     ",
        r"    ( {f} )  ",
        r"     \_/     ",
        r"     `\`     ",
    ],
    idle: &[
        &[
            r"    \\|//    ",
            r"     \|/     ",
            r"    ( {f} )  ",
            r"     \_/     ",
            r"     `\`     ",
        ],
        &[
            r"    \\|//    ",
            r"     |\|     ",
            r"    ( {f} )  ",
            r"     \_/     ",
            r"     `\`     ",
        ],
    ],
    active: &[
        &[
            r"    \\|//    ",
            r"     \|/     ",
            r"  d-( {f} )  ",
            r"     \_/     ",
            r"     `\`     ",
        ],
        &[
            r"    \\|//    ",
            r"     \|/     ",
            r"    ( {f} )-b",
            r"     \_/     ",
            r"     `\`     ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: GENTLE_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"   \|/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"   ` `   ",
        ],
        run_a: &[
            r"   \|/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d/\b   ",
        ],
        run_b: &[
            r"   \|/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"   b/\d  ",
        ],
        jump: &[
            r"   \|/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  _   _  ",
        ],
        fall: &[
            r"   \|/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d   b  ",
        ],
        duck_a: &[
            r"         ",
            r"   \\|// ",
            r"  d( {f} )",
            r"   \_/_b ",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"  \\|//  ",
            r"  ( {f} )b",
            r"  d_\_/  ",
            r"         ",
        ],
        crashed: &[
            r"   \x/   ",
            r"   \|/   ",
            r"  ( {f} )",
            r"   \_/   ",
            r"  d   b  ",
        ],
    },
};

const VINEKITH: MonsterForm = MonsterForm {
    small: &[
        r"   \\\|///    ",
        r"    \\|//     ",
        r"  ( ( {f} ) ) ",
        r"   \\v_v//    ",
        r"   d/   \b    ",
    ],
    idle: &[
        &[
            r"    \\\|///    ",
            r"     \\|//     ",
            r"   ( ( {f} ) ) ",
            r"    \\v_v//    ",
            r"    /|/^\|\    ",
            r"    d/   \b    ",
        ],
        &[
            r"     \\|//     ",
            r"    \\\|///    ",
            r"   ( ( {f} ) ) ",
            r"    \\v_v//    ",
            r"    /|\^/|\    ",
            r"    d/   \b    ",
        ],
    ],
    active: &[
        &[
            r"   \\\\|////   ",
            r"    \\\|///    ",
            r"   ( ( {f} ) ) ",
            r"    \\v_v//    ",
            r"    /|/^\|\    ",
            r"    d_/   \_b  ",
        ],
        &[
            r"    \\\|///    ",
            r"   \\\\|////   ",
            r"   ( ( {f} ) ) ",
            r"    \\v_v//    ",
            r"    /|\^/|\    ",
            r"     _d   b_   ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: WALKING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r" \\\|/// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d/   \b ",
        ],
        run_a: &[
            r" \\\|/// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/  \b ",
        ],
        run_b: &[
            r" \\\|/// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d/  \_b ",
        ],
        jump: &[
            r" \\\|/// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\v_v// ",
            r"  _   _  ",
        ],
        fall: &[
            r" \\\|/// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_   _b ",
        ],
        duck_a: &[
            r"         ",
            r"\\\|/// /",
            r"( ({f}) )",
            r"d\\v_v//b",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"\ \\\|///",
            r"( ({f}) )",
            r"b\\v_v//d",
            r"         ",
        ],
        crashed: &[
            r" \\x|x// ",
            r"  \\|//  ",
            r"( ({f}) )",
            r" \\_x_// ",
            r" d_   _b ",
        ],
    },
};

const ELDROAK: MonsterForm = MonsterForm {
    small: &[
        r"   \\\\|||////    ",
        r"    \\\|||///     ",
        r"  (( ( {f} ) ))   ",
        r"   \\\\v_v////    ",
        r"    \\\___///     ",
        r"     d_/ \_b      ",
    ],
    idle: &[
        &[
            r"    \\\\\|||/////    ",
            r"     \\\\|||////     ",
            r"    ((  ___  ))      ",
            r"   (( ( {f} ) ))     ",
            r"    \\\\v_v////      ",
            r"     \\\\___////     ",
            r"     ||\\\v///||     ",
            r"     ||  d|b  ||     ",
            r"     ||_______||     ",
        ],
        &[
            r"    \\\\\|||/////    ",
            r"     \\\\|||////     ",
            r"    ((  ___  ))      ",
            r"   (( ( {f} ) ))     ",
            r"    \\\\v_v////      ",
            r"     \\\\___////     ",
            r"     ||/\\v//\||     ",
            r"     ||  d|b  ||     ",
            r"     ||_______||     ",
        ],
    ],
    active: &[
        &[
            r"   \\\\\\|||//////   ",
            r"     \\\\|||////     ",
            r"    ((  ___  ))      ",
            r"   (( ( {f} ) ))     ",
            r"    \\\\v_v////      ",
            r"     \\\\___////     ",
            r"     ||\\\v///||     ",
            r"     ||  d b  ||     ",
            r"     ||_______||     ",
        ],
        &[
            r"     \\\\\|||/////   ",
            r"    \\\\\\|||//////  ",
            r"    ((  ___  ))      ",
            r"   (( ( {f} ) ))     ",
            r"    \\\\v_v////      ",
            r"     \\\\___////     ",
            r"     ||/\\v//\||     ",
            r"     ||  d b  ||     ",
            r"     ||_______||     ",
        ],
    ],
    idle_motion: IDLE_MOTION,
    active_motion: WALKING_MOTION,
    game: GameSpriteSet {
        waiting: &[
            r"\\\|||///",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/ \_b ",
        ],
        run_a: &[
            r"\\\|||///",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d__/\_b ",
        ],
        run_b: &[
            r"\\\|||///",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_/\__b ",
        ],
        jump: &[
            r"\\\|||///",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\v_v// ",
            r"  _   _  ",
        ],
        fall: &[
            r"\\\|||///",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\v_v// ",
            r" d_   _b ",
        ],
        duck_a: &[
            r"         ",
            r"\\\|||///",
            r"( ({f}) )",
            r"d\\v_v//b",
            r"         ",
        ],
        duck_b: &[
            r"         ",
            r"///|||\\\",
            r"( ({f}) )",
            r"b\\v_v//d",
            r"         ",
        ],
        crashed: &[
            r"\\x|x|x//",
            r" \\|||// ",
            r"( ({f}) )",
            r" \\_x_// ",
            r" d_   _b ",
        ],
    },
};

// ── Mood, status, CLI helpers (unchanged behaviour) ──────────────────────────

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

pub fn personality_text(monster: &Monster) -> String {
    match classify_mood(monster) {
        MoodState::Tired => format!("{} semble epuise...", monster.name),
        MoodState::Hungry => format!("{} a faim.", monster.name),
        MoodState::Sad => format!("{} est triste.", monster.name),
        MoodState::Proud => format!("{} est fier de toi !", monster.name),
        MoodState::Fine => format!("{} va bien.", monster.name),
    }
}

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

pub fn render_status(monster: &Monster, xp_gained: u32) {
    let art = ascii_art(monster);
    println!();
    for line in &art {
        println!("  {}", line.bright_magenta());
    }
    println!();
    println!(
        "  {} — {} {} {} {} {}",
        monster.name.bold(),
        format!("lv.{}", monster.level).bright_yellow(),
        "·".bright_black(),
        monster.species.form_name(monster.stage).bright_cyan(),
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
