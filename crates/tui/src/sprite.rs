//! Pixel-art pet sprite rendered with truecolor half-block characters.
//!
//! Each terminal cell packs two vertical pixels: the upper pixel becomes the
//! glyph foreground and the lower pixel the background of a `▀` (upper half
//! block). Transparent pixels fall back to the terminal background, which keeps
//! the creature sitting cleanly on the dark pet panel.
//!
//! The body is authored once as a character grid; expressions only repaint the
//! small eye and mouth region, and emotion particles are drawn in the rows
//! above the head. This mirrors how real sprite sheets are built and keeps the
//! art editable.

use commitchi_pet::{Mood, Reaction};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Sprite width in pixels (also the number of terminal columns it occupies).
pub const WIDTH: usize = 18;

const UPPER_HALF: &str = "▀";
const LOWER_HALF: &str = "▄";

/// Base creature body, no face. Face pixels are repainted by the overlay.
///
/// Legend: `.` transparent, `k` outline, `d` dark orange, `o` orange,
/// `O` light orange (highlight), `c` cream belly, `C` cream highlight.
const BODY: [&str; 24] = [
    "...kkk......kkk...",
    "...kok......kok...",
    "...kok......kok...",
    "..kkokk....kkokk..",
    "..koook....koook..",
    ".kkodokkkkkkodokk.",
    ".koodOOkOokoodook.",
    ".koOOOOOOOooooook.",
    ".kkkkOOOOOOookkkk.",
    "..kkOOOOOOooookk..",
    "..kOOOOOOoooddok..",
    "..kOOOOOoooddddk..",
    ".kkOOOOoooooddokk.",
    "kkOOOOoCCccoooookk",
    "kooOOoCCCCccoooook",
    "koooooCCCCccoooook",
    "kkkoddCCCCccoookkk",
    "..koddcCCcccoook..",
    "..koddccccccoook..",
    "..kkooccccccookk..",
    "...kkooccccookk...",
    "...koooooooodk....",
    "...kookkookddk....",
    "...kkkkkkkkkkk....",
];

const SPRITE_HEIGHT: usize = BODY.len();

fn palette(ch: char) -> Option<Color> {
    let rgb = match ch {
        '.' | ' ' => return None,
        'k' => (40, 22, 16),
        'd' => (150, 70, 35),
        'o' => (212, 105, 55),
        'O' => (240, 150, 95),
        'c' => (242, 214, 175),
        'C' => (250, 232, 205),
        'p' => (228, 132, 112),
        'e' => (28, 18, 14),
        'w' => (250, 250, 250),
        'm' => (120, 55, 32),
        _ => (255, 0, 255), // loud magenta surfaces an unknown legend char
    };
    Some(Color::Rgb(rgb.0, rgb.1, rgb.2))
}

/// Visible emotional states. Derived from the pet's mood and current reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expression {
    Neutral,
    Happy,
    Excited,
    Curious,
    Confused,
    Wincing,
    Anxious,
    Sad,
    Sleepy,
}

impl Expression {
    /// Reactions to a diff take priority; otherwise the persisted mood drives
    /// the resting expression.
    pub fn from_mood_reaction(mood: Mood, reaction: Reaction) -> Self {
        match reaction {
            Reaction::Excited => Expression::Excited,
            Reaction::Curious => Expression::Curious,
            Reaction::Confused => Expression::Confused,
            Reaction::Wincing => Expression::Wincing,
            Reaction::Nodding => Expression::Happy,
            Reaction::Calm => match mood {
                Mood::Thriving => Expression::Happy,
                Mood::Content => Expression::Happy,
                Mood::Neutral => Expression::Neutral,
                Mood::Anxious => Expression::Anxious,
                Mood::Sulking => Expression::Sad,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Eyes {
    Open,
    Wide,
    Happy,
    Sad,
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mouth {
    Neutral,
    Smile,
    Open,
    Frown,
}

/// Eye column centers and their shared rows.
const EYE_ROW_TOP: usize = 9;
const EYE_ROW_BOTTOM: usize = 10;
const LEFT_EYE: usize = 5;
const RIGHT_EYE: usize = 11;
const MOUTH_ROW: usize = 13;

fn eyes_for(expr: Expression) -> Eyes {
    match expr {
        Expression::Neutral => Eyes::Open,
        Expression::Happy => Eyes::Happy,
        Expression::Excited => Eyes::Wide,
        Expression::Curious => Eyes::Wide,
        Expression::Confused => Eyes::Open,
        Expression::Wincing => Eyes::Closed,
        Expression::Anxious => Eyes::Wide,
        Expression::Sad => Eyes::Sad,
        Expression::Sleepy => Eyes::Closed,
    }
}

fn mouth_for(expr: Expression) -> Mouth {
    match expr {
        Expression::Neutral | Expression::Sleepy => Mouth::Neutral,
        Expression::Happy => Mouth::Smile,
        Expression::Excited => Mouth::Open,
        Expression::Curious | Expression::Confused | Expression::Anxious => Mouth::Neutral,
        Expression::Wincing | Expression::Sad => Mouth::Frown,
    }
}

fn draw_eye(grid: &mut [Vec<char>], col: usize, eyes: Eyes, mirror: bool) {
    // `col` is the left pixel of a 2-wide eye; `hi`/`lo` are its columns with
    // the white highlight kept toward the creature's outer edge.
    let outer = if mirror { col + 1 } else { col };
    let inner = if mirror { col } else { col + 1 };
    match eyes {
        Eyes::Open => {
            grid[EYE_ROW_TOP][outer] = 'w';
            grid[EYE_ROW_TOP][inner] = 'e';
            grid[EYE_ROW_BOTTOM][outer] = 'e';
            grid[EYE_ROW_BOTTOM][inner] = 'e';
        }
        Eyes::Wide => {
            grid[EYE_ROW_TOP - 1][outer] = 'e';
            grid[EYE_ROW_TOP - 1][inner] = 'e';
            grid[EYE_ROW_TOP][outer] = 'w';
            grid[EYE_ROW_TOP][inner] = 'e';
            grid[EYE_ROW_BOTTOM][outer] = 'e';
            grid[EYE_ROW_BOTTOM][inner] = 'e';
        }
        Eyes::Happy => {
            // upward curve "^"
            grid[EYE_ROW_BOTTOM][outer] = 'e';
            grid[EYE_ROW_TOP][inner] = 'e';
        }
        Eyes::Sad => {
            // downward, droopy
            grid[EYE_ROW_TOP][outer] = 'e';
            grid[EYE_ROW_BOTTOM][inner] = 'e';
        }
        Eyes::Closed => {
            grid[EYE_ROW_BOTTOM][outer] = 'e';
            grid[EYE_ROW_BOTTOM][inner] = 'e';
        }
    }
}

fn draw_mouth(grid: &mut [Vec<char>], mouth: Mouth) {
    let (a, b) = (8usize, 9usize);
    match mouth {
        Mouth::Neutral => {
            grid[MOUTH_ROW][a] = 'm';
            grid[MOUTH_ROW][b] = 'm';
        }
        Mouth::Smile => {
            grid[MOUTH_ROW][a - 1] = 'm';
            grid[MOUTH_ROW + 1][a] = 'm';
            grid[MOUTH_ROW + 1][b] = 'm';
            grid[MOUTH_ROW][b + 1] = 'm';
        }
        Mouth::Open => {
            grid[MOUTH_ROW][a] = 'm';
            grid[MOUTH_ROW][b] = 'm';
            grid[MOUTH_ROW + 1][a] = 'p';
            grid[MOUTH_ROW + 1][b] = 'p';
        }
        Mouth::Frown => {
            grid[MOUTH_ROW + 1][a - 1] = 'm';
            grid[MOUTH_ROW][a] = 'm';
            grid[MOUTH_ROW][b] = 'm';
            grid[MOUTH_ROW + 1][b + 1] = 'm';
        }
    }
}

fn has_blush(expr: Expression) -> bool {
    matches!(expr, Expression::Happy | Expression::Excited)
}

fn draw_blush(grid: &mut [Vec<char>]) {
    grid[EYE_ROW_BOTTOM + 1][LEFT_EYE - 1] = 'p';
    grid[EYE_ROW_BOTTOM + 1][RIGHT_EYE + 1] = 'p';
}

fn base_grid() -> Vec<Vec<char>> {
    BODY.iter().map(|row| row.chars().collect()).collect()
}

/// Build the pixel grid for an expression, optionally with eyes blinked shut.
fn expression_grid(expr: Expression, blink: bool) -> Vec<Vec<char>> {
    let mut grid = base_grid();
    let eyes = if blink { Eyes::Closed } else { eyes_for(expr) };
    draw_eye(&mut grid, LEFT_EYE, eyes, false);
    draw_eye(&mut grid, RIGHT_EYE, eyes, true);
    if has_blush(expr) {
        draw_blush(&mut grid);
    }
    draw_mouth(&mut grid, mouth_for(expr));
    grid
}

/// Render a pixel grid into half-block terminal lines.
fn render_grid(grid: &[Vec<char>]) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(grid.len().div_ceil(2));
    let mut row = 0;
    while row < grid.len() {
        let top = &grid[row];
        let empty = Vec::new();
        let bottom = grid.get(row + 1).unwrap_or(&empty);
        let mut spans = Vec::with_capacity(WIDTH);
        for col in 0..WIDTH {
            let top_color = top.get(col).copied().and_then(palette);
            let bottom_color = bottom.get(col).copied().and_then(palette);
            spans.push(half_block_span(top_color, bottom_color));
        }
        lines.push(Line::from(spans));
        row += 2;
    }
    lines
}

fn half_block_span(top: Option<Color>, bottom: Option<Color>) -> Span<'static> {
    match (top, bottom) {
        (Some(t), Some(b)) => Span::styled(UPPER_HALF, Style::default().fg(t).bg(b)),
        (Some(t), None) => Span::styled(UPPER_HALF, Style::default().fg(t)),
        (None, Some(b)) => Span::styled(LOWER_HALF, Style::default().fg(b)),
        (None, None) => Span::raw(" "),
    }
}

/// Emotion particles drawn above the head, animated by frame phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Particle {
    None,
    Sparkles,
    Hearts,
    Question,
    Swirl,
    Sweat,
    Sleep,
}

impl Particle {
    pub fn for_expression(expr: Expression) -> Self {
        match expr {
            Expression::Excited => Particle::Sparkles,
            Expression::Happy => Particle::Hearts,
            Expression::Curious => Particle::Question,
            Expression::Confused => Particle::Swirl,
            Expression::Wincing | Expression::Anxious => Particle::Sweat,
            Expression::Sad | Expression::Sleepy => Particle::Sleep,
            Expression::Neutral => Particle::None,
        }
    }
}

fn particle_glyphs(particle: Particle) -> Option<([&'static str; 2], Color)> {
    let value = match particle {
        Particle::None => return None,
        Particle::Sparkles => (["  ✦      ✧  ", "    ✧  ✦    "], Color::Rgb(250, 220, 90)),
        Particle::Hearts => (["    ♥      ", "       ♥   "], Color::Rgb(235, 90, 110)),
        Particle::Question => (["        ?  ", "        ?  "], Color::Rgb(180, 200, 240)),
        Particle::Swirl => (["       @   ", "       ?   "], Color::Rgb(190, 160, 220)),
        Particle::Sweat => (["          ,", "          '"], Color::Rgb(130, 190, 230)),
        Particle::Sleep => (["       z   ", "      z    "], Color::Rgb(170, 180, 200)),
    };
    Some(value)
}

fn particle_line(particle: Particle, phase: usize) -> Line<'static> {
    match particle_glyphs(particle) {
        None => Line::from(" ".repeat(WIDTH)),
        Some((glyphs, color)) => {
            let glyph = glyphs[phase % glyphs.len()];
            Line::from(Span::styled(
                format!("{glyph:<width$}", width = WIDTH),
                Style::default().fg(color),
            ))
        }
    }
}

/// A fully composed sprite frame: a particle line on top of the creature.
pub fn frame(expr: Expression, blink: bool, particle_phase: usize) -> Vec<Line<'static>> {
    let particle = Particle::for_expression(expr);
    let mut lines = Vec::with_capacity(SPRITE_HEIGHT / 2 + 1);
    lines.push(particle_line(particle, particle_phase));
    lines.extend(render_grid(&expression_grid(expr, blink)));
    lines
}

/// Label shown beside an expression in the stdout preview.
pub const PREVIEW_EXPRESSIONS: [(&str, Expression); 9] = [
    ("neutral", Expression::Neutral),
    ("happy", Expression::Happy),
    ("excited", Expression::Excited),
    ("curious", Expression::Curious),
    ("confused", Expression::Confused),
    ("wincing", Expression::Wincing),
    ("anxious", Expression::Anxious),
    ("sad", Expression::Sad),
    ("sleepy", Expression::Sleepy),
];

fn rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 0, 255),
    }
}

fn ansi_cell(top: Option<Color>, bottom: Option<Color>) -> String {
    match (top, bottom) {
        (Some(t), Some(b)) => {
            let (tr, tg, tb) = rgb(t);
            let (br, bg, bb) = rgb(b);
            format!("\x1b[38;2;{tr};{tg};{tb};48;2;{br};{bg};{bb}m{UPPER_HALF}\x1b[0m")
        }
        (Some(t), None) => {
            let (r, g, b) = rgb(t);
            format!("\x1b[38;2;{r};{g};{b}m{UPPER_HALF}\x1b[0m")
        }
        (None, Some(b)) => {
            let (r, g, b) = rgb(b);
            format!("\x1b[38;2;{r};{g};{b}m{LOWER_HALF}\x1b[0m")
        }
        (None, None) => " ".to_owned(),
    }
}

/// Render a frame as raw ANSI strings for stdout previews. The live TUI uses
/// [`frame`] instead; this exists only so the sprite can be eyeballed quickly
/// outside the alternate screen.
pub fn preview_lines(expr: Expression, blink: bool, particle_phase: usize) -> Vec<String> {
    let mut out = Vec::new();

    let particle = Particle::for_expression(expr);
    out.push(match particle_glyphs(particle) {
        None => " ".repeat(WIDTH),
        Some((glyphs, color)) => {
            let (r, g, b) = rgb(color);
            let glyph = glyphs[particle_phase % glyphs.len()];
            format!(
                "\x1b[38;2;{r};{g};{b}m{glyph:<width$}\x1b[0m",
                width = WIDTH
            )
        }
    });

    let grid = expression_grid(expr, blink);
    let mut row = 0;
    while row < grid.len() {
        let top = &grid[row];
        let bottom = grid.get(row + 1);
        let mut line = String::new();
        for col in 0..WIDTH {
            let top_color = top.get(col).copied().and_then(palette);
            let bottom_color = bottom
                .and_then(|cells| cells.get(col).copied())
                .and_then(palette);
            line.push_str(&ansi_cell(top_color, bottom_color));
        }
        out.push(line);
        row += 2;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_rows_are_uniform_width() {
        for (index, row) in BODY.iter().enumerate() {
            assert_eq!(
                row.chars().count(),
                WIDTH,
                "BODY row {index} is not {WIDTH} wide: {row:?}"
            );
        }
        assert_eq!(SPRITE_HEIGHT % 2, 0, "sprite height must be even");
    }

    #[test]
    fn every_body_pixel_has_a_palette_entry() {
        for row in BODY {
            for ch in row.chars() {
                if ch == '.' {
                    continue;
                }
                assert_ne!(
                    palette(ch),
                    Some(Color::Rgb(255, 0, 255)),
                    "unknown legend char {ch:?}"
                );
            }
        }
    }

    #[test]
    fn frame_has_expected_line_count() {
        // one particle line plus half the (even) sprite height.
        let lines = frame(Expression::Neutral, false, 0);
        assert_eq!(lines.len(), 1 + SPRITE_HEIGHT / 2);
    }

    #[test]
    fn expression_mapping_prioritises_reactions() {
        assert_eq!(
            Expression::from_mood_reaction(Mood::Sulking, Reaction::Excited),
            Expression::Excited
        );
        assert_eq!(
            Expression::from_mood_reaction(Mood::Thriving, Reaction::Calm),
            Expression::Happy
        );
        assert_eq!(
            Expression::from_mood_reaction(Mood::Neutral, Reaction::Calm),
            Expression::Neutral
        );
    }

    #[test]
    fn blink_closes_eyes_regardless_of_expression() {
        let open = expression_grid(Expression::Excited, false);
        let blinked = expression_grid(Expression::Excited, true);
        assert_ne!(open, blinked);
    }
}
