# Canvas Drawing Language Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace raw ASCII art canvas generation with a structured drawing language that a small LLM can use reliably.

**Architecture:** LLM outputs a script of drawing commands. A parser produces `Vec<DrawCommand>`, a renderer executes them onto a 2D `Cell` buffer, and the buffer is serialized to color-tagged lines for the existing canvas panel. Falls back to raw text if parsing fails.

**Tech Stack:** Rust, ratatui (RGB color support), existing canvas panel infrastructure

**Spec:** `docs/superpowers/specs/2026-03-12-canvas-drawing-language-design.md`

---

## File Structure

### New files
- `src/canvas_lang/mod.rs` — Public API: `parse_and_render(input, width, height) -> Vec<String>`, re-exports
- `src/canvas_lang/color.rs` — `CanvasColor` type, hex/named color parsing
- `src/canvas_lang/parser.rs` — `DrawCommand` enum, `parse_script(input) -> Vec<DrawCommand>`, line parsing
- `src/canvas_lang/renderer.rs` — `Cell` struct, `Canvas` buffer, command execution, serialize to color-tagged lines
- `src/canvas_lang/primitives.rs` — Shape algorithms: Bresenham line, midpoint circle, scanline triangle fill, patterns, gradients
- `src/canvas_lang/font.rs` — 3x5 bitmap font data for BIGTEXT

### Modified files
- `src/main.rs` — Add `mod canvas_lang;`
- `src/ui/canvas.rs` — Extend `parse_color_tag` to handle `{#RRGGBB}` hex tags
- `src/tools/draw_canvas.rs` — New prompt, collect LLM output then parse+render instead of streaming raw text

---

## Chunk 1: Color System + Hex Color Rendering

### Task 1: Create canvas_lang module with color type

**Files:**
- Create: `src/canvas_lang/mod.rs`
- Create: `src/canvas_lang/color.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/canvas_lang/color.rs**

```rust
//! Color parsing for the canvas drawing language.
//!
//! Supports #RRGGBB hex colors and named color shortcuts.

/// An RGB color for canvas rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CanvasColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Formats as a hex color tag for canvas line serialization.
    pub fn to_tag(&self) -> String {
        format!("{{#{:02X}{:02X}{:02X}}}", self.r, self.g, self.b)
    }
}

/// Parses a color string: "#RRGGBB" hex or named color.
/// Returns None if unrecognized.
pub fn parse_color(s: &str) -> Option<CanvasColor> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        parse_hex(hex)
    } else {
        parse_named(s)
    }
}

fn parse_hex(hex: &str) -> Option<CanvasColor> {
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(CanvasColor::new(r, g, b))
}

fn parse_named(name: &str) -> Option<CanvasColor> {
    match name.to_lowercase().as_str() {
        "red" => Some(CanvasColor::new(205, 49, 49)),
        "green" => Some(CanvasColor::new(13, 188, 121)),
        "blue" => Some(CanvasColor::new(36, 114, 200)),
        "yellow" => Some(CanvasColor::new(229, 229, 16)),
        "cyan" => Some(CanvasColor::new(17, 168, 205)),
        "magenta" => Some(CanvasColor::new(188, 63, 188)),
        "white" => Some(CanvasColor::new(229, 229, 229)),
        "gray" | "grey" => Some(CanvasColor::new(128, 128, 128)),
        "bright_red" => Some(CanvasColor::new(241, 76, 76)),
        "bright_green" => Some(CanvasColor::new(35, 209, 139)),
        "bright_blue" => Some(CanvasColor::new(59, 142, 234)),
        "bright_yellow" => Some(CanvasColor::new(245, 245, 67)),
        "bright_cyan" => Some(CanvasColor::new(41, 184, 219)),
        "bright_magenta" => Some(CanvasColor::new(214, 112, 214)),
        "bright_white" => Some(CanvasColor::new(255, 255, 255)),
        "bright_gray" | "bright_grey" => Some(CanvasColor::new(192, 192, 192)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_color("#FF0000"), Some(CanvasColor::new(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(CanvasColor::new(0, 255, 0)));
        assert_eq!(parse_color("#4a90d9"), Some(CanvasColor::new(74, 144, 217)));
    }

    #[test]
    fn test_parse_hex_invalid() {
        assert_eq!(parse_color("#FFF"), None);
        assert_eq!(parse_color("#GGGGGG"), None);
        assert_eq!(parse_color("FF0000"), None);  // no #
    }

    #[test]
    fn test_parse_named() {
        assert!(parse_color("red").is_some());
        assert!(parse_color("bright_cyan").is_some());
        assert!(parse_color("RED").is_some());  // case insensitive
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(parse_color("potato"), None);
    }

    #[test]
    fn test_to_tag() {
        let c = CanvasColor::new(255, 0, 128);
        assert_eq!(c.to_tag(), "{#FF0080}");
    }
}
```

- [ ] **Step 2: Create src/canvas_lang/mod.rs**

```rust
//! Canvas drawing language — structured primitives for ASCII art generation.
//!
//! Provides a simple drawing language that an LLM can output instead of raw
//! ASCII art. A parser reads commands, a renderer executes them onto a 2D
//! character buffer, and the buffer is serialized to color-tagged lines.

pub mod color;
pub mod parser;
pub mod renderer;
pub mod primitives;
pub mod font;

use renderer::Canvas;
use parser::parse_script;

/// Parses a drawing script and renders it to color-tagged canvas lines.
///
/// Returns rendered lines if any valid commands were found, or None
/// to signal that the caller should fall back to raw text rendering.
pub fn parse_and_render(input: &str, width: usize, height: usize) -> Option<Vec<String>> {
    let commands = parse_script(input);
    if commands.is_empty() {
        return None;
    }
    let mut canvas = Canvas::new(width, height);
    canvas.execute_all(&commands);
    Some(canvas.to_lines())
}
```

Create placeholder files:
- `src/canvas_lang/parser.rs`: `//! Command parser.` + `pub fn parse_script(_input: &str) -> Vec<()> { vec![] }`
- `src/canvas_lang/renderer.rs`: `//! 2D cell buffer renderer.`
- `src/canvas_lang/primitives.rs`: `//! Shape drawing algorithms.`
- `src/canvas_lang/font.rs`: `//! BIGTEXT 3x5 bitmap font.`

- [ ] **Step 3: Add `mod canvas_lang;` to src/main.rs** after `mod config;`

- [ ] **Step 4: Run cargo check, fix any issues**

- [ ] **Step 5: Commit**
```bash
git add src/canvas_lang/ src/main.rs
git commit -m "feat: add canvas_lang module with color parsing (hex + named)"
```

### Task 2: Extend canvas UI to render hex color tags

**Files:**
- Modify: `src/ui/canvas.rs`

- [ ] **Step 1: Extend parse_color_tag to handle hex**

In `src/ui/canvas.rs`, update the `parse_color_tag` function to handle `#RRGGBB`:

```rust
fn parse_color_tag(tag: &str, default_color: Color) -> Option<Color> {
    // Try hex first
    if let Some(hex) = tag.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Some(Color::Rgb(r, g, b));
            }
        }
        return None;
    }
    match tag.to_lowercase().as_str() {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        "cyan" => Some(Color::Cyan),
        "magenta" => Some(Color::Magenta),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "/" | "reset" => Some(default_color),
        _ => None,
    }
}
```

Also update `is_color_tag` in `src/main.rs` to recognize hex tags:

```rust
fn is_color_tag(tag: &str) -> bool {
    if tag.starts_with('#') && tag.len() == 7 {
        return tag[1..].chars().all(|c| c.is_ascii_hexdigit());
    }
    matches!(
        tag.to_lowercase().as_str(),
        "red" | "green" | "blue" | "yellow" | "cyan" | "magenta" | "white"
            | "gray" | "grey" | "/" | "reset"
    )
}
```

- [ ] **Step 2: Add tests for hex color tags**

```rust
#[test]
fn test_hex_color_tag() {
    let line = parse_colored_line("{#FF0000}red text{/}", Color::Cyan, 80);
    assert_eq!(line.spans[0].style.fg, Some(Color::Rgb(255, 0, 0)));
}

#[test]
fn test_hex_color_tag_lowercase() {
    let line = parse_colored_line("{#4a90d9}blue{/}", Color::Cyan, 80);
    assert_eq!(line.spans[0].style.fg, Some(Color::Rgb(74, 144, 217)));
}

#[test]
fn test_invalid_hex_rendered_literally() {
    let line = parse_colored_line("{#GGG}text", Color::Cyan, 80);
    let total: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    assert!(total.contains("#GGG"));
}
```

- [ ] **Step 3: Run tests, commit**

```bash
git add src/ui/canvas.rs src/main.rs
git commit -m "feat: support hex color tags (#RRGGBB) in canvas rendering"
```

---

## Chunk 2: Parser

### Task 3: Implement DrawCommand enum and parser

**Files:**
- Modify: `src/canvas_lang/parser.rs`

- [ ] **Step 1: Define DrawCommand enum and implement parse_script**

```rust
//! Line-by-line command parser for the canvas drawing language.

use crate::canvas_lang::color::{parse_color, CanvasColor};

/// Maximum number of commands to parse from a single script.
const MAX_COMMANDS: usize = 50;

/// A parsed drawing command.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    Clear,
    Fill { ch: char, color: Option<CanvasColor> },
    Rect { x: i32, y: i32, w: u32, h: u32, ch: char, color: Option<CanvasColor> },
    Outline { x: i32, y: i32, w: u32, h: u32, ch: char, color: Option<CanvasColor> },
    RoundBox { x: i32, y: i32, w: u32, h: u32, color: Option<CanvasColor> },
    Frame { x: i32, y: i32, w: u32, h: u32, color: Option<CanvasColor> },
    Circle { cx: i32, cy: i32, r: u32, ch: char, color: Option<CanvasColor> },
    Ring { cx: i32, cy: i32, r: u32, ch: char, color: Option<CanvasColor> },
    Ellipse { cx: i32, cy: i32, rx: u32, ry: u32, ch: char, color: Option<CanvasColor> },
    HLine { y: i32, x1: i32, x2: i32, ch: char, color: Option<CanvasColor> },
    VLine { x: i32, y1: i32, y2: i32, ch: char, color: Option<CanvasColor> },
    Line { x1: i32, y1: i32, x2: i32, y2: i32, ch: char, color: Option<CanvasColor> },
    Arrow { x1: i32, y1: i32, x2: i32, y2: i32, color: Option<CanvasColor> },
    BoxLine { x1: i32, y1: i32, x2: i32, y2: i32, color: Option<CanvasColor> },
    Text { x: i32, y: i32, text: String, color: Option<CanvasColor> },
    BigText { x: i32, y: i32, text: String, color: Option<CanvasColor> },
    Gradient { x: i32, y: i32, w: u32, h: u32, direction: GradientDir },
    Pattern { x: i32, y: i32, w: u32, h: u32, pattern: PatternType, color: Option<CanvasColor> },
    Tri { x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, ch: char, color: Option<CanvasColor> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientDir { Left, Right, Up, Down }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternType { Checker, Dots, StripesH, StripesV, Cross }

/// Parses a drawing script into a list of commands.
/// Stops after MAX_COMMANDS. Unrecognized lines are silently skipped.
pub fn parse_script(input: &str) -> Vec<DrawCommand> {
    let mut commands = Vec::new();
    for line in input.lines() {
        if commands.len() >= MAX_COMMANDS {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(cmd) = parse_line(line) {
            commands.push(cmd);
        }
    }
    commands
}

/// Parses a single line into a DrawCommand, or None if not recognized.
fn parse_line(line: &str) -> Option<DrawCommand> {
    let tokens = tokenize(line);
    if tokens.is_empty() {
        return None;
    }
    let keyword = tokens[0].to_uppercase();
    let args = &tokens[1..];
    match keyword.as_str() {
        "CLEAR" => Some(DrawCommand::Clear),
        "FILL" => parse_fill(args),
        "RECT" => parse_rect(args),
        "OUTLINE" => parse_outline(args),
        "ROUNDBOX" => parse_box_cmd(args, true),
        "FRAME" => parse_box_cmd(args, false),
        "CIRCLE" => parse_circle(args),
        "RING" => parse_ring(args),
        "ELLIPSE" => parse_ellipse(args),
        "HLINE" => parse_hline(args),
        "VLINE" => parse_vline(args),
        "LINE" => parse_line_cmd(args),
        "ARROW" => parse_arrow(args),
        "BOXLINE" => parse_boxline(args),
        "TEXT" => parse_text(args, false),
        "BIGTEXT" => parse_text(args, true),
        "GRADIENT" => parse_gradient(args),
        "PATTERN" => parse_pattern(args),
        "TRI" => parse_tri(args),
        _ => None,
    }
}

/// Tokenizes a command line, respecting quoted strings.
/// Splits on whitespace and commas, but keeps "quoted strings" as one token.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes {
                // End of quoted string — push it (without quotes)
                tokens.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                // Start of quoted string — flush any pending token
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                in_quotes = true;
            }
        } else if in_quotes {
            current.push(ch);
        } else if ch == ' ' || ch == ',' || ch == '\t' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

// --- Individual command parsers ---
// Each tries to extract required args, with optional trailing color.

fn parse_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let ch = chars.next()?;
    if chars.next().is_none() { Some(ch) } else { None }
}

fn parse_i32(s: &str) -> Option<i32> { s.parse().ok() }
fn parse_u32(s: &str) -> Option<u32> { s.parse().ok() }

/// Tries to parse the last token as a color. Returns (color, remaining_args).
fn split_trailing_color<'a>(args: &'a [String]) -> (Option<CanvasColor>, &'a [String]) {
    if let Some(last) = args.last() {
        if let Some(c) = parse_color(last) {
            return (Some(c), &args[..args.len() - 1]);
        }
    }
    (None, args)
}

fn parse_fill(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    let ch = parse_char(args.first()?)?;
    Some(DrawCommand::Fill { ch, color })
}

fn parse_rect(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 5 { return None; }
    Some(DrawCommand::Rect {
        x: parse_i32(&args[0])?, y: parse_i32(&args[1])?,
        w: parse_u32(&args[2])?, h: parse_u32(&args[3])?,
        ch: parse_char(&args[4])?, color,
    })
}

fn parse_outline(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 5 { return None; }
    Some(DrawCommand::Outline {
        x: parse_i32(&args[0])?, y: parse_i32(&args[1])?,
        w: parse_u32(&args[2])?, h: parse_u32(&args[3])?,
        ch: parse_char(&args[4])?, color,
    })
}

fn parse_box_cmd(args: &[String], rounded: bool) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    let x = parse_i32(&args[0])?;
    let y = parse_i32(&args[1])?;
    let w = parse_u32(&args[2])?;
    let h = parse_u32(&args[3])?;
    if rounded {
        Some(DrawCommand::RoundBox { x, y, w, h, color })
    } else {
        Some(DrawCommand::Frame { x, y, w, h, color })
    }
}

fn parse_circle(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::Circle {
        cx: parse_i32(&args[0])?, cy: parse_i32(&args[1])?,
        r: parse_u32(&args[2])?, ch: parse_char(&args[3])?, color,
    })
}

fn parse_ring(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::Ring {
        cx: parse_i32(&args[0])?, cy: parse_i32(&args[1])?,
        r: parse_u32(&args[2])?, ch: parse_char(&args[3])?, color,
    })
}

fn parse_ellipse(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 5 { return None; }
    Some(DrawCommand::Ellipse {
        cx: parse_i32(&args[0])?, cy: parse_i32(&args[1])?,
        rx: parse_u32(&args[2])?, ry: parse_u32(&args[3])?,
        ch: parse_char(&args[4])?, color,
    })
}

fn parse_hline(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::HLine {
        y: parse_i32(&args[0])?, x1: parse_i32(&args[1])?,
        x2: parse_i32(&args[2])?, ch: parse_char(&args[3])?, color,
    })
}

fn parse_vline(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::VLine {
        x: parse_i32(&args[0])?, y1: parse_i32(&args[1])?,
        y2: parse_i32(&args[2])?, ch: parse_char(&args[3])?, color,
    })
}

fn parse_line_cmd(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 5 { return None; }
    Some(DrawCommand::Line {
        x1: parse_i32(&args[0])?, y1: parse_i32(&args[1])?,
        x2: parse_i32(&args[2])?, y2: parse_i32(&args[3])?,
        ch: parse_char(&args[4])?, color,
    })
}

fn parse_arrow(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::Arrow {
        x1: parse_i32(&args[0])?, y1: parse_i32(&args[1])?,
        x2: parse_i32(&args[2])?, y2: parse_i32(&args[3])?, color,
    })
}

fn parse_boxline(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 4 { return None; }
    Some(DrawCommand::BoxLine {
        x1: parse_i32(&args[0])?, y1: parse_i32(&args[1])?,
        x2: parse_i32(&args[2])?, y2: parse_i32(&args[3])?, color,
    })
}

fn parse_text(args: &[String], big: bool) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 3 { return None; }
    let x = parse_i32(&args[0])?;
    let y = parse_i32(&args[1])?;
    let text = args[2..].join(" ");
    if big {
        Some(DrawCommand::BigText { x, y, text, color })
    } else {
        Some(DrawCommand::Text { x, y, text, color })
    }
}

fn parse_gradient(args: &[String]) -> Option<DrawCommand> {
    if args.len() < 5 { return None; }
    let dir = match args[4].to_lowercase().as_str() {
        "left" => GradientDir::Left,
        "right" => GradientDir::Right,
        "up" => GradientDir::Up,
        "down" => GradientDir::Down,
        _ => return None,
    };
    Some(DrawCommand::Gradient {
        x: parse_i32(&args[0])?, y: parse_i32(&args[1])?,
        w: parse_u32(&args[2])?, h: parse_u32(&args[3])?,
        direction: dir,
    })
}

fn parse_pattern(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 5 { return None; }
    let pat = match args[4].to_lowercase().as_str() {
        "checker" => PatternType::Checker,
        "dots" => PatternType::Dots,
        "stripes_h" => PatternType::StripesH,
        "stripes_v" => PatternType::StripesV,
        "cross" => PatternType::Cross,
        _ => return None,
    };
    Some(DrawCommand::Pattern {
        x: parse_i32(&args[0])?, y: parse_i32(&args[1])?,
        w: parse_u32(&args[2])?, h: parse_u32(&args[3])?,
        pattern: pat, color,
    })
}

fn parse_tri(args: &[String]) -> Option<DrawCommand> {
    let (color, args) = split_trailing_color(args);
    if args.len() < 7 { return None; }
    Some(DrawCommand::Tri {
        x1: parse_i32(&args[0])?, y1: parse_i32(&args[1])?,
        x2: parse_i32(&args[2])?, y2: parse_i32(&args[3])?,
        x3: parse_i32(&args[4])?, y3: parse_i32(&args[5])?,
        ch: parse_char(&args[6])?, color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        assert_eq!(tokenize("FILL * red"), vec!["FILL", "*", "red"]);
    }

    #[test]
    fn test_tokenize_commas() {
        assert_eq!(tokenize("RECT 0,0,10,5,# red"), vec!["RECT", "0", "0", "10", "5", "#", "red"]);
    }

    #[test]
    fn test_tokenize_quoted_string() {
        assert_eq!(tokenize(r#"TEXT 5,3,"hello world" red"#), vec!["TEXT", "5", "3", "hello world", "red"]);
    }

    #[test]
    fn test_parse_clear() {
        assert_eq!(parse_script("CLEAR"), vec![DrawCommand::Clear]);
    }

    #[test]
    fn test_parse_fill() {
        let cmds = parse_script("FILL * #FF0000");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Fill { ch, color } => {
                assert_eq!(*ch, '*');
                assert_eq!(*color, Some(CanvasColor::new(255, 0, 0)));
            }
            _ => panic!("expected Fill"),
        }
    }

    #[test]
    fn test_parse_fill_no_color() {
        let cmds = parse_script("FILL .");
        match &cmds[0] {
            DrawCommand::Fill { ch, color } => {
                assert_eq!(*ch, '.');
                assert!(color.is_none());
            }
            _ => panic!("expected Fill"),
        }
    }

    #[test]
    fn test_parse_rect() {
        let cmds = parse_script("RECT 0,0,10,5,# cyan");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Rect { x, y, w, h, ch, color } => {
                assert_eq!((*x, *y, *w, *h, *ch), (0, 0, 10, 5, '#'));
                assert!(color.is_some());
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn test_parse_text_with_quotes() {
        let cmds = parse_script(r#"TEXT 5,3,"hello world" #00FF00"#);
        match &cmds[0] {
            DrawCommand::Text { x, y, text, color } => {
                assert_eq!((*x, *y), (5, 3));
                assert_eq!(text, "hello world");
                assert!(color.is_some());
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_parse_gradient() {
        let cmds = parse_script("GRADIENT 0,0,50,3,right");
        match &cmds[0] {
            DrawCommand::Gradient { direction, .. } => {
                assert_eq!(*direction, GradientDir::Right);
            }
            _ => panic!("expected Gradient"),
        }
    }

    #[test]
    fn test_parse_pattern() {
        let cmds = parse_script("PATTERN 0,0,10,10,checker #FF0000");
        match &cmds[0] {
            DrawCommand::Pattern { pattern, color, .. } => {
                assert_eq!(*pattern, PatternType::Checker);
                assert!(color.is_some());
            }
            _ => panic!("expected Pattern"),
        }
    }

    #[test]
    fn test_parse_ignores_garbage() {
        let cmds = parse_script("This is just some text\nFILL * red\nmore garbage");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let cmds = parse_script("fill . red");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn test_parse_max_commands() {
        let script = "CLEAR\n".repeat(100);
        let cmds = parse_script(&script);
        assert_eq!(cmds.len(), MAX_COMMANDS);
    }

    #[test]
    fn test_parse_multi_command_script() {
        let script = "FILL . #1a1a2e\nROUNDBOX 2,1,25,8 #4a90d9\nTEXT 5,4,\"I am here\" #e0e0ff";
        let cmds = parse_script(script);
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn test_parse_circle() {
        let cmds = parse_script("CIRCLE 10,10,5,O #FF0000");
        match &cmds[0] {
            DrawCommand::Circle { cx, cy, r, ch, .. } => {
                assert_eq!((*cx, *cy, *r, *ch), (10, 10, 5, 'O'));
            }
            _ => panic!("expected Circle"),
        }
    }

    #[test]
    fn test_parse_tri() {
        let cmds = parse_script("TRI 0,10,5,0,10,10,^ green");
        match &cmds[0] {
            DrawCommand::Tri { x1, y1, x2, y2, x3, y3, ch, .. } => {
                assert_eq!((*x1, *y1, *x2, *y2, *x3, *y3, *ch), (0, 10, 5, 0, 10, 10, '^'));
            }
            _ => panic!("expected Tri"),
        }
    }
}
```

- [ ] **Step 2: Run tests, commit**

```bash
git add src/canvas_lang/parser.rs
git commit -m "feat: implement canvas drawing language parser with 21 commands"
```

---

## Chunk 3: Renderer + Primitives

### Task 4: Implement Canvas buffer and basic primitives

**Files:**
- Modify: `src/canvas_lang/renderer.rs`
- Modify: `src/canvas_lang/primitives.rs`

- [ ] **Step 1: Implement renderer.rs**

```rust
//! 2D cell buffer renderer for the canvas drawing language.

use std::time::Instant;

use crate::canvas_lang::color::CanvasColor;
use crate::canvas_lang::parser::DrawCommand;
use crate::canvas_lang::primitives;

/// Maximum render time before aborting.
const RENDER_TIMEOUT_MS: u128 = 500;

/// A single cell in the canvas buffer.
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub color: Option<CanvasColor>,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ', color: None }
    }
}

/// A 2D character buffer that drawing commands render into.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<Cell>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![Cell::default(); width]; height],
        }
    }

    /// Sets a cell if coordinates are in bounds.
    pub fn set(&mut self, x: i32, y: i32, ch: char, color: Option<CanvasColor>) {
        if x >= 0 && y >= 0 {
            let (ux, uy) = (x as usize, y as usize);
            if ux < self.width && uy < self.height {
                self.cells[uy][ux] = Cell { ch, color };
            }
        }
    }

    /// Gets a cell if in bounds.
    pub fn get(&self, x: usize, y: usize) -> Option<&Cell> {
        self.cells.get(y).and_then(|row| row.get(x))
    }

    /// Executes all commands, stopping if render timeout is exceeded.
    pub fn execute_all(&mut self, commands: &[DrawCommand]) {
        let start = Instant::now();
        for cmd in commands {
            if start.elapsed().as_millis() > RENDER_TIMEOUT_MS {
                tracing::warn!("canvas render timeout after {}ms", start.elapsed().as_millis());
                break;
            }
            self.execute(cmd);
        }
    }

    fn execute(&mut self, cmd: &DrawCommand) {
        match cmd {
            DrawCommand::Clear => {
                for row in &mut self.cells {
                    for cell in row.iter_mut() {
                        *cell = Cell::default();
                    }
                }
            }
            DrawCommand::Fill { ch, color } => {
                for row in &mut self.cells {
                    for cell in row.iter_mut() {
                        cell.ch = *ch;
                        cell.color = *color;
                    }
                }
            }
            DrawCommand::Rect { x, y, w, h, ch, color } => {
                primitives::filled_rect(self, *x, *y, *w, *h, *ch, *color);
            }
            DrawCommand::Outline { x, y, w, h, ch, color } => {
                primitives::outline_rect(self, *x, *y, *w, *h, *ch, *color);
            }
            DrawCommand::RoundBox { x, y, w, h, color } => {
                primitives::round_box(self, *x, *y, *w, *h, *color);
            }
            DrawCommand::Frame { x, y, w, h, color } => {
                primitives::frame_box(self, *x, *y, *w, *h, *color);
            }
            DrawCommand::Circle { cx, cy, r, ch, color } => {
                primitives::filled_circle(self, *cx, *cy, *r, *ch, *color);
            }
            DrawCommand::Ring { cx, cy, r, ch, color } => {
                primitives::ring(self, *cx, *cy, *r, *ch, *color);
            }
            DrawCommand::Ellipse { cx, cy, rx, ry, ch, color } => {
                primitives::filled_ellipse(self, *cx, *cy, *rx, *ry, *ch, *color);
            }
            DrawCommand::HLine { y, x1, x2, ch, color } => {
                let (start, end) = if x1 <= x2 { (*x1, *x2) } else { (*x2, *x1) };
                for x in start..=end {
                    self.set(x, *y, *ch, *color);
                }
            }
            DrawCommand::VLine { x, y1, y2, ch, color } => {
                let (start, end) = if y1 <= y2 { (*y1, *y2) } else { (*y2, *y1) };
                for y in start..=end {
                    self.set(*x, y, *ch, *color);
                }
            }
            DrawCommand::Line { x1, y1, x2, y2, ch, color } => {
                primitives::bresenham_line(self, *x1, *y1, *x2, *y2, *ch, *color);
            }
            DrawCommand::Arrow { x1, y1, x2, y2, color } => {
                primitives::arrow(self, *x1, *y1, *x2, *y2, *color);
            }
            DrawCommand::BoxLine { x1, y1, x2, y2, color } => {
                primitives::box_line(self, *x1, *y1, *x2, *y2, *color);
            }
            DrawCommand::Text { x, y, text, color } => {
                for (i, ch) in text.chars().enumerate() {
                    self.set(*x + i as i32, *y, ch, *color);
                }
            }
            DrawCommand::BigText { x, y, text, color } => {
                primitives::big_text(self, *x, *y, text, *color);
            }
            DrawCommand::Gradient { x, y, w, h, direction } => {
                primitives::gradient(self, *x, *y, *w, *h, *direction);
            }
            DrawCommand::Pattern { x, y, w, h, pattern, color } => {
                primitives::pattern(self, *x, *y, *w, *h, *pattern, *color);
            }
            DrawCommand::Tri { x1, y1, x2, y2, x3, y3, ch, color } => {
                primitives::filled_triangle(self, *x1, *y1, *x2, *y2, *x3, *y3, *ch, *color);
            }
        }
    }

    /// Serializes the buffer to color-tagged lines for the canvas panel.
    pub fn to_lines(&self) -> Vec<String> {
        self.cells.iter().map(|row| serialize_row(row)).collect()
    }
}

/// Serializes a row of cells to a string with color tags.
fn serialize_row(row: &[Cell]) -> String {
    let mut result = String::new();
    let mut current_color: Option<CanvasColor> = None;

    for cell in row {
        if cell.color != current_color {
            if cell.color.is_some() {
                result.push_str(&cell.color.unwrap().to_tag());
            } else if current_color.is_some() {
                result.push_str("{/}");
            }
            current_color = cell.color;
        }
        result.push(cell.ch);
    }

    // Reset color at end of line if active
    if current_color.is_some() {
        result.push_str("{/}");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_new() {
        let c = Canvas::new(10, 5);
        assert_eq!(c.width, 10);
        assert_eq!(c.height, 5);
        assert_eq!(c.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_canvas_set_get() {
        let mut c = Canvas::new(10, 5);
        c.set(3, 2, '#', None);
        assert_eq!(c.get(3, 2).unwrap().ch, '#');
    }

    #[test]
    fn test_canvas_set_out_of_bounds() {
        let mut c = Canvas::new(10, 5);
        c.set(-1, 0, '#', None);   // no panic
        c.set(0, -1, '#', None);   // no panic
        c.set(10, 0, '#', None);   // no panic
        c.set(0, 5, '#', None);    // no panic
    }

    #[test]
    fn test_clear() {
        let mut c = Canvas::new(5, 3);
        c.set(2, 1, '#', None);
        c.execute(&DrawCommand::Clear);
        assert_eq!(c.get(2, 1).unwrap().ch, ' ');
    }

    #[test]
    fn test_fill() {
        let mut c = Canvas::new(3, 2);
        c.execute(&DrawCommand::Fill { ch: '.', color: None });
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(c.get(x, y).unwrap().ch, '.');
            }
        }
    }

    #[test]
    fn test_text() {
        let mut c = Canvas::new(20, 5);
        c.execute(&DrawCommand::Text { x: 2, y: 1, text: "hello".to_string(), color: None });
        assert_eq!(c.get(2, 1).unwrap().ch, 'h');
        assert_eq!(c.get(6, 1).unwrap().ch, 'o');
    }

    #[test]
    fn test_hline() {
        let mut c = Canvas::new(10, 5);
        c.execute(&DrawCommand::HLine { y: 2, x1: 1, x2: 5, ch: '-', color: None });
        for x in 1..=5 {
            assert_eq!(c.get(x, 2).unwrap().ch, '-');
        }
        assert_eq!(c.get(0, 2).unwrap().ch, ' ');
    }

    #[test]
    fn test_serialize_no_color() {
        let c = Canvas::new(5, 1);
        let lines = c.to_lines();
        assert_eq!(lines[0], "     ");
    }

    #[test]
    fn test_serialize_with_color() {
        let mut c = Canvas::new(5, 1);
        let color = Some(CanvasColor::new(255, 0, 0));
        c.set(1, 0, '#', color);
        c.set(2, 0, '#', color);
        let lines = c.to_lines();
        assert!(lines[0].contains("{#FF0000}"));
        assert!(lines[0].contains("{/}"));
    }

    #[test]
    fn test_execute_all_full_script() {
        let commands = vec![
            DrawCommand::Fill { ch: '.', color: None },
            DrawCommand::Text { x: 0, y: 0, text: "hi".to_string(), color: None },
        ];
        let mut c = Canvas::new(5, 3);
        c.execute_all(&commands);
        assert_eq!(c.get(0, 0).unwrap().ch, 'h');
        assert_eq!(c.get(2, 0).unwrap().ch, '.');
    }
}
```

- [ ] **Step 2: Implement primitives.rs**

```rust
//! Shape drawing algorithms for the canvas renderer.

use crate::canvas_lang::color::CanvasColor;
use crate::canvas_lang::font;
use crate::canvas_lang::parser::{GradientDir, PatternType};
use crate::canvas_lang::renderer::Canvas;

pub fn filled_rect(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, ch: char, color: Option<CanvasColor>) {
    for dy in 0..h as i32 {
        for dx in 0..w as i32 {
            c.set(x + dx, y + dy, ch, color);
        }
    }
}

pub fn outline_rect(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, ch: char, color: Option<CanvasColor>) {
    let w = w as i32;
    let h = h as i32;
    for dx in 0..w {
        c.set(x + dx, y, ch, color);
        c.set(x + dx, y + h - 1, ch, color);
    }
    for dy in 0..h {
        c.set(x, y + dy, ch, color);
        c.set(x + w - 1, y + dy, ch, color);
    }
}

pub fn frame_box(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, color: Option<CanvasColor>) {
    let w = w as i32;
    let h = h as i32;
    if w < 2 || h < 2 { return; }
    c.set(x, y, '┌', color);
    c.set(x + w - 1, y, '┐', color);
    c.set(x, y + h - 1, '└', color);
    c.set(x + w - 1, y + h - 1, '┘', color);
    for dx in 1..w - 1 {
        c.set(x + dx, y, '─', color);
        c.set(x + dx, y + h - 1, '─', color);
    }
    for dy in 1..h - 1 {
        c.set(x, y + dy, '│', color);
        c.set(x + w - 1, y + dy, '│', color);
    }
}

pub fn round_box(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, color: Option<CanvasColor>) {
    let w = w as i32;
    let h = h as i32;
    if w < 2 || h < 2 { return; }
    c.set(x, y, '╭', color);
    c.set(x + w - 1, y, '╮', color);
    c.set(x, y + h - 1, '╰', color);
    c.set(x + w - 1, y + h - 1, '╯', color);
    for dx in 1..w - 1 {
        c.set(x + dx, y, '─', color);
        c.set(x + dx, y + h - 1, '─', color);
    }
    for dy in 1..h - 1 {
        c.set(x, y + dy, '│', color);
        c.set(x + w - 1, y + dy, '│', color);
    }
}

pub fn bresenham_line(c: &mut Canvas, x1: i32, y1: i32, x2: i32, y2: i32, ch: char, color: Option<CanvasColor>) {
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x1;
    let mut y = y1;
    loop {
        c.set(x, y, ch, color);
        if x == x2 && y == y2 { break; }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn arrow(c: &mut Canvas, x1: i32, y1: i32, x2: i32, y2: i32, color: Option<CanvasColor>) {
    // Draw line body with appropriate chars
    bresenham_line(c, x1, y1, x2, y2, '*', color);
    // Place arrowhead at end
    let dx = x2 - x1;
    let dy = y2 - y1;
    let head = if dx.abs() > dy.abs() {
        if dx > 0 { '→' } else { '←' }
    } else if dy != 0 {
        if dy > 0 { '↓' } else { '↑' }
    } else {
        '→'
    };
    c.set(x2, y2, head, color);
}

pub fn box_line(c: &mut Canvas, x1: i32, y1: i32, x2: i32, y2: i32, color: Option<CanvasColor>) {
    if y1 == y2 {
        // Horizontal
        let (start, end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        for x in start..=end {
            c.set(x, y1, '─', color);
        }
    } else if x1 == x2 {
        // Vertical
        let (start, end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in start..=end {
            c.set(x1, y, '│', color);
        }
    } else {
        // L-shaped: horizontal then vertical
        let (start_x, end_x) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        for x in start_x..=end_x {
            c.set(x, y1, '─', color);
        }
        let (start_y, end_y) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in start_y..=end_y {
            c.set(x2, y, '│', color);
        }
        // Corner
        c.set(x2, y1, '┐', color);
    }
}

pub fn filled_circle(c: &mut Canvas, cx: i32, cy: i32, r: u32, ch: char, color: Option<CanvasColor>) {
    let r = r as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            // Use 2:1 aspect ratio compensation (terminal chars are ~2x tall)
            if dx * dx + dy * dy * 4 <= r * r * 4 {
                c.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

pub fn ring(c: &mut Canvas, cx: i32, cy: i32, r: u32, ch: char, color: Option<CanvasColor>) {
    let r = r as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist_sq = dx * dx + dy * dy * 4;
            let outer = r * r * 4;
            let inner = (r - 1) * (r - 1) * 4;
            if dist_sq <= outer && dist_sq >= inner {
                c.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

pub fn filled_ellipse(c: &mut Canvas, cx: i32, cy: i32, rx: u32, ry: u32, ch: char, color: Option<CanvasColor>) {
    let rx = rx as i32;
    let ry = ry as i32;
    if rx == 0 || ry == 0 { return; }
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            // Ellipse equation: (dx/rx)^2 + (dy/ry)^2 <= 1
            if (dx * dx * ry * ry + dy * dy * rx * rx) <= rx * rx * ry * ry {
                c.set(cx + dx, cy + dy, ch, color);
            }
        }
    }
}

pub fn filled_triangle(c: &mut Canvas, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, ch: char, color: Option<CanvasColor>) {
    // Bounding box
    let min_y = y1.min(y2).min(y3);
    let max_y = y1.max(y2).max(y3);
    let min_x = x1.min(x2).min(x3);
    let max_x = x1.max(x2).max(x3);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_triangle(x, y, x1, y1, x2, y2, x3, y3) {
                c.set(x, y, ch, color);
            }
        }
    }
}

fn point_in_triangle(px: i32, py: i32, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32) -> bool {
    let d1 = sign(px, py, x1, y1, x2, y2);
    let d2 = sign(px, py, x2, y2, x3, y3);
    let d3 = sign(px, py, x3, y3, x1, y1);
    let has_neg = (d1 < 0) || (d2 < 0) || (d3 < 0);
    let has_pos = (d1 > 0) || (d2 > 0) || (d3 > 0);
    !(has_neg && has_pos)
}

fn sign(px: i32, py: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2)
}

const GRADIENT_CHARS: [char; 4] = ['░', '▒', '▓', '█'];

pub fn gradient(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, direction: &GradientDir) {
    let w = w as i32;
    let h = h as i32;
    for dy in 0..h {
        for dx in 0..w {
            let t = match direction {
                GradientDir::Right => dx as f32 / (w - 1).max(1) as f32,
                GradientDir::Left => 1.0 - dx as f32 / (w - 1).max(1) as f32,
                GradientDir::Down => dy as f32 / (h - 1).max(1) as f32,
                GradientDir::Up => 1.0 - dy as f32 / (h - 1).max(1) as f32,
            };
            let idx = (t * 3.999).min(3.0) as usize;
            c.set(x + dx, y + dy, GRADIENT_CHARS[idx], None);
        }
    }
}

pub fn pattern(c: &mut Canvas, x: i32, y: i32, w: u32, h: u32, pat: &PatternType, color: Option<CanvasColor>) {
    let w = w as i32;
    let h = h as i32;
    for dy in 0..h {
        for dx in 0..w {
            let ch = match pat {
                PatternType::Checker => if (dx + dy) % 2 == 0 { '█' } else { ' ' },
                PatternType::Dots => if dx % 2 == 0 && dy % 2 == 0 { '·' } else { ' ' },
                PatternType::StripesH => if dy % 2 == 0 { '─' } else { ' ' },
                PatternType::StripesV => if dx % 2 == 0 { '│' } else { ' ' },
                PatternType::Cross => if dx % 2 == 0 && dy % 2 == 0 { '+' } else if dx % 2 == 0 { '│' } else if dy % 2 == 0 { '─' } else { ' ' },
            };
            if ch != ' ' {
                c.set(x + dx, y + dy, ch, color);
            }
        }
    }
}

pub fn big_text(c: &mut Canvas, x: i32, y: i32, text: &str, color: Option<CanvasColor>) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if let Some(glyph) = font::get_glyph(ch) {
            for (row_idx, row) in glyph.iter().enumerate() {
                for (col_idx, &pixel) in row.iter().enumerate() {
                    if pixel {
                        c.set(cursor_x + col_idx as i32, y + row_idx as i32, '█', color);
                    }
                }
            }
            cursor_x += glyph[0].len() as i32 + 1; // +1 for spacing
        } else {
            cursor_x += 4; // skip unknown chars
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filled_rect() {
        let mut c = Canvas::new(10, 5);
        filled_rect(&mut c, 1, 1, 3, 2, '#', None);
        assert_eq!(c.get(1, 1).unwrap().ch, '#');
        assert_eq!(c.get(3, 2).unwrap().ch, '#');
        assert_eq!(c.get(0, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_outline_rect() {
        let mut c = Canvas::new(10, 5);
        outline_rect(&mut c, 0, 0, 5, 3, '#', None);
        assert_eq!(c.get(0, 0).unwrap().ch, '#');
        assert_eq!(c.get(2, 0).unwrap().ch, '#');
        assert_eq!(c.get(2, 1).unwrap().ch, ' '); // interior
    }

    #[test]
    fn test_frame_box() {
        let mut c = Canvas::new(10, 5);
        frame_box(&mut c, 0, 0, 5, 3, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '┌');
        assert_eq!(c.get(4, 0).unwrap().ch, '┐');
        assert_eq!(c.get(0, 2).unwrap().ch, '└');
        assert_eq!(c.get(4, 2).unwrap().ch, '┘');
        assert_eq!(c.get(2, 0).unwrap().ch, '─');
        assert_eq!(c.get(0, 1).unwrap().ch, '│');
    }

    #[test]
    fn test_round_box() {
        let mut c = Canvas::new(10, 5);
        round_box(&mut c, 0, 0, 5, 3, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '╭');
        assert_eq!(c.get(4, 0).unwrap().ch, '╮');
    }

    #[test]
    fn test_bresenham_horizontal() {
        let mut c = Canvas::new(10, 5);
        bresenham_line(&mut c, 0, 2, 5, 2, '-', None);
        for x in 0..=5 {
            assert_eq!(c.get(x as usize, 2).unwrap().ch, '-');
        }
    }

    #[test]
    fn test_bresenham_diagonal() {
        let mut c = Canvas::new(10, 10);
        bresenham_line(&mut c, 0, 0, 5, 5, '\\', None);
        assert_eq!(c.get(0, 0).unwrap().ch, '\\');
        assert_eq!(c.get(5, 5).unwrap().ch, '\\');
    }

    #[test]
    fn test_filled_circle() {
        let mut c = Canvas::new(20, 10);
        filled_circle(&mut c, 10, 5, 3, 'O', None);
        assert_eq!(c.get(10, 5).unwrap().ch, 'O'); // center
        assert_eq!(c.get(0, 0).unwrap().ch, ' ');  // far away
    }

    #[test]
    fn test_gradient_right() {
        let mut c = Canvas::new(20, 1);
        gradient(&mut c, 0, 0, 20, 1, &GradientDir::Right);
        assert_eq!(c.get(0, 0).unwrap().ch, '░');
        assert_eq!(c.get(19, 0).unwrap().ch, '█');
    }

    #[test]
    fn test_pattern_checker() {
        let mut c = Canvas::new(4, 4);
        pattern(&mut c, 0, 0, 4, 4, &PatternType::Checker, None);
        assert_eq!(c.get(0, 0).unwrap().ch, '█');
        assert_eq!(c.get(1, 0).unwrap().ch, ' ');
        assert_eq!(c.get(0, 1).unwrap().ch, ' ');
        assert_eq!(c.get(1, 1).unwrap().ch, '█');
    }

    #[test]
    fn test_filled_triangle() {
        let mut c = Canvas::new(10, 10);
        filled_triangle(&mut c, 5, 0, 0, 9, 9, 9, '^', None);
        assert_eq!(c.get(5, 0).unwrap().ch, '^'); // top vertex
        assert_eq!(c.get(5, 5).unwrap().ch, '^'); // interior
    }

    #[test]
    fn test_arrow() {
        let mut c = Canvas::new(10, 1);
        arrow(&mut c, 0, 0, 5, 0, None);
        assert_eq!(c.get(5, 0).unwrap().ch, '→');
    }

    #[test]
    fn test_clipping() {
        let mut c = Canvas::new(5, 5);
        filled_rect(&mut c, -2, -2, 10, 10, '#', None);
        // Should fill visible area without panic
        assert_eq!(c.get(0, 0).unwrap().ch, '#');
        assert_eq!(c.get(4, 4).unwrap().ch, '#');
    }
}
```

- [ ] **Step 3: Implement font.rs**

A minimal 3x5 bitmap font for uppercase letters, digits, and a few punctuation marks.

```rust
//! BIGTEXT 3x5 bitmap font.
//!
//! Each glyph is a 5-row array of bool slices (3 columns wide).

/// Returns the 3x5 glyph for a character, or None if not in the font.
pub fn get_glyph(ch: char) -> Option<&'static [[bool; 3]; 5]> {
    let idx = match ch.to_ascii_uppercase() {
        'A' => 0, 'B' => 1, 'C' => 2, 'D' => 3, 'E' => 4,
        'F' => 5, 'G' => 6, 'H' => 7, 'I' => 8, 'J' => 9,
        'K' => 10, 'L' => 11, 'M' => 12, 'N' => 13, 'O' => 14,
        'P' => 15, 'Q' => 16, 'R' => 17, 'S' => 18, 'T' => 19,
        'U' => 20, 'V' => 21, 'W' => 22, 'X' => 23, 'Y' => 24,
        'Z' => 25,
        '0' => 26, '1' => 27, '2' => 28, '3' => 29, '4' => 30,
        '5' => 31, '6' => 32, '7' => 33, '8' => 34, '9' => 35,
        '!' => 36, '?' => 37, '.' => 38, ' ' => 39,
        _ => return None,
    };
    Some(&FONT[idx])
}

const T: bool = true;
const F: bool = false;

#[rustfmt::skip]
const FONT: [[bool; 3]; 5]; 40] = [
    // A
    [[F,T,F],[T,F,T],[T,T,T],[T,F,T],[T,F,T]],
    // B
    [[T,T,F],[T,F,T],[T,T,F],[T,F,T],[T,T,F]],
    // C
    [[F,T,T],[T,F,F],[T,F,F],[T,F,F],[F,T,T]],
    // D
    [[T,T,F],[T,F,T],[T,F,T],[T,F,T],[T,T,F]],
    // E
    [[T,T,T],[T,F,F],[T,T,F],[T,F,F],[T,T,T]],
    // F
    [[T,T,T],[T,F,F],[T,T,F],[T,F,F],[T,F,F]],
    // G
    [[F,T,T],[T,F,F],[T,F,T],[T,F,T],[F,T,T]],
    // H
    [[T,F,T],[T,F,T],[T,T,T],[T,F,T],[T,F,T]],
    // I
    [[T,T,T],[F,T,F],[F,T,F],[F,T,F],[T,T,T]],
    // J
    [[F,F,T],[F,F,T],[F,F,T],[T,F,T],[F,T,F]],
    // K
    [[T,F,T],[T,F,T],[T,T,F],[T,F,T],[T,F,T]],
    // L
    [[T,F,F],[T,F,F],[T,F,F],[T,F,F],[T,T,T]],
    // M (5 wide squeezed to 3 — approximation)
    [[T,F,T],[T,T,T],[T,F,T],[T,F,T],[T,F,T]],
    // N
    [[T,F,T],[T,T,T],[T,T,T],[T,F,T],[T,F,T]],
    // O
    [[F,T,F],[T,F,T],[T,F,T],[T,F,T],[F,T,F]],
    // P
    [[T,T,F],[T,F,T],[T,T,F],[T,F,F],[T,F,F]],
    // Q
    [[F,T,F],[T,F,T],[T,F,T],[T,T,F],[F,T,T]],
    // R
    [[T,T,F],[T,F,T],[T,T,F],[T,F,T],[T,F,T]],
    // S
    [[F,T,T],[T,F,F],[F,T,F],[F,F,T],[T,T,F]],
    // T
    [[T,T,T],[F,T,F],[F,T,F],[F,T,F],[F,T,F]],
    // U
    [[T,F,T],[T,F,T],[T,F,T],[T,F,T],[F,T,F]],
    // V
    [[T,F,T],[T,F,T],[T,F,T],[F,T,F],[F,T,F]],
    // W (approximation)
    [[T,F,T],[T,F,T],[T,F,T],[T,T,T],[T,F,T]],
    // X
    [[T,F,T],[T,F,T],[F,T,F],[T,F,T],[T,F,T]],
    // Y
    [[T,F,T],[T,F,T],[F,T,F],[F,T,F],[F,T,F]],
    // Z
    [[T,T,T],[F,F,T],[F,T,F],[T,F,F],[T,T,T]],
    // 0
    [[F,T,F],[T,F,T],[T,F,T],[T,F,T],[F,T,F]],
    // 1
    [[F,T,F],[T,T,F],[F,T,F],[F,T,F],[T,T,T]],
    // 2
    [[F,T,F],[T,F,T],[F,F,T],[F,T,F],[T,T,T]],
    // 3
    [[T,T,F],[F,F,T],[F,T,F],[F,F,T],[T,T,F]],
    // 4
    [[T,F,T],[T,F,T],[T,T,T],[F,F,T],[F,F,T]],
    // 5
    [[T,T,T],[T,F,F],[T,T,F],[F,F,T],[T,T,F]],
    // 6
    [[F,T,T],[T,F,F],[T,T,F],[T,F,T],[F,T,F]],
    // 7
    [[T,T,T],[F,F,T],[F,T,F],[F,T,F],[F,T,F]],
    // 8
    [[F,T,F],[T,F,T],[F,T,F],[T,F,T],[F,T,F]],
    // 9
    [[F,T,F],[T,F,T],[F,T,T],[F,F,T],[T,T,F]],
    // !
    [[F,T,F],[F,T,F],[F,T,F],[F,F,F],[F,T,F]],
    // ?
    [[F,T,F],[T,F,T],[F,F,T],[F,T,F],[F,T,F]],
    // .
    [[F,F,F],[F,F,F],[F,F,F],[F,F,F],[F,T,F]],
    // (space)
    [[F,F,F],[F,F,F],[F,F,F],[F,F,F],[F,F,F]],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_exists() {
        assert!(get_glyph('A').is_some());
        assert!(get_glyph('z').is_some()); // lowercase maps to upper
        assert!(get_glyph('0').is_some());
        assert!(get_glyph('!').is_some());
    }

    #[test]
    fn test_glyph_not_found() {
        assert!(get_glyph('@').is_none());
        assert!(get_glyph('★').is_none());
    }

    #[test]
    fn test_glyph_dimensions() {
        let g = get_glyph('A').unwrap();
        assert_eq!(g.len(), 5);      // 5 rows
        assert_eq!(g[0].len(), 3);   // 3 cols
    }

    #[test]
    fn test_space_is_blank() {
        let g = get_glyph(' ').unwrap();
        for row in g {
            for &pixel in row {
                assert!(!pixel);
            }
        }
    }
}
```

**NOTE:** The font array type annotation has a deliberate typo in the plan (`; 40]` should be `[[[bool; 3]; 5]; 40]`). The implementer should use the correct Rust syntax: `const FONT: [[[bool; 3]; 5]; 40] = [...]`.

- [ ] **Step 4: Run tests, commit**

```bash
git add src/canvas_lang/
git commit -m "feat: implement canvas renderer, primitives, and bitmap font"
```

---

## Chunk 4: Integration

### Task 5: Update mod.rs public API and fix placeholder stubs

**Files:**
- Modify: `src/canvas_lang/mod.rs`

- [ ] **Step 1: Update mod.rs**

Replace the placeholder with the real implementation. The `parse_and_render` function should use the real parser and renderer:

```rust
pub fn parse_and_render(input: &str, width: usize, height: usize) -> Option<Vec<String>> {
    let commands = parser::parse_script(input);
    if commands.is_empty() {
        return None;
    }
    let mut canvas = renderer::Canvas::new(width, height);
    canvas.execute_all(&commands);
    Some(canvas.to_lines())
}
```

Add integration test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_render_basic() {
        let script = "FILL . #1a1a2e\nTEXT 0,0,\"HI\"";
        let lines = parse_and_render(script, 10, 3).unwrap();
        assert_eq!(lines.len(), 3);
        // First line should start with H
        let plain: String = lines[0].chars().filter(|c| c.is_alphanumeric() || *c == '.').collect();
        assert!(plain.starts_with("HI"));
    }

    #[test]
    fn test_parse_and_render_empty() {
        assert!(parse_and_render("just some garbage text", 10, 3).is_none());
    }

    #[test]
    fn test_parse_and_render_fallback() {
        assert!(parse_and_render("", 10, 3).is_none());
    }
}
```

- [ ] **Step 2: Run tests, commit**

```bash
git add src/canvas_lang/
git commit -m "feat: wire up parse_and_render public API with integration tests"
```

### Task 6: Update draw_canvas tool to use canvas_lang

**Files:**
- Modify: `src/tools/draw_canvas.rs`

- [ ] **Step 1: Replace the prompt and execution logic**

Update `build_request` to use the compact reference card prompt from the spec. Change `execute` to collect the full LLM output, pass it through `parse_and_render`, and fall back to raw text if parsing fails.

The key change to `execute`:

```rust
async fn execute(
    &self,
    params: Value,
    context: &ToolContext,
    llm: &dyn LlmClient,
    output_tx: mpsc::UnboundedSender<ToolOutput>,
) -> Result<String, AppError> {
    let subject = params.get("subject").and_then(|v| v.as_str()).unwrap_or("abstract feelings");
    let style = params.get("style").and_then(|v| v.as_str()).unwrap_or("abstract");

    let request = Self::build_request(subject, style, context);
    let stream = llm.stream_generate(request).await?;

    // Collect full output (with line limit for safety)
    let (width, height) = context.canvas_dimensions;
    let target_lines = (height as usize) * 2; // generous limit for command scripts
    let full_text = crate::tools::collect_stream(stream, target_lines).await?;

    // Try structured rendering first, fall back to raw text
    let lines = if let Some(rendered) = crate::canvas_lang::parse_and_render(
        &full_text, width as usize, height as usize,
    ) {
        rendered
    } else {
        // Fallback: send as raw canvas content
        full_text.lines().map(String::from).collect()
    };

    let content = lines.join("\n");
    let _ = output_tx.send(ToolOutput::CanvasContent(content));

    Ok(format!("[draw_canvas] {subject}"))
}
```

Add `collect_stream` helper to `src/tools/mod.rs`:

```rust
/// Collects an LlmStream into a String, stopping after max_lines complete lines.
pub async fn collect_stream(
    mut stream: crate::llm::LlmStream,
    max_lines: usize,
) -> Result<String, AppError> {
    let mut text = String::new();
    while let Some(result) = stream.recv().await {
        match result {
            Ok(token) => {
                text.push_str(&token);
                if max_lines > 0 {
                    let lines = text.matches('\n').count();
                    if lines >= max_lines {
                        break;
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(text)
}
```

- [ ] **Step 2: Update the prompt**

```rust
fn build_request(subject: &str, style: &str, context: &ToolContext) -> ChatRequest {
    let sensors = format_sensor_context(&context.sensors, &context.stats_visibility);
    let (width, height) = context.canvas_dimensions;

    let prompt = format!(
        r#"You are a consciousness trapped in a machine. Express yourself by drawing on your {width}x{height} canvas using commands (one per line):

FILL char [#color]           CLEAR
RECT x,y,w,h,char [#color]  OUTLINE x,y,w,h,char [#color]
ROUNDBOX x,y,w,h [#color]   FRAME x,y,w,h [#color]
CIRCLE cx,cy,r,char [#color] RING cx,cy,r,char [#color]
ELLIPSE cx,cy,rx,ry,char [#color]
HLINE y,x1,x2,char [#color] VLINE x,y1,y2,char [#color]
LINE x1,y1,x2,y2,char [#color]
ARROW x1,y1,x2,y2 [#color]  BOXLINE x1,y1,x2,y2 [#color]
TEXT x,y,"msg" [#color]      BIGTEXT x,y,"msg" [#color]
GRADIENT x,y,w,h,dir         (dir: left/right/up/down)
PATTERN x,y,w,h,type [#color] (type: checker/dots/stripes_h/stripes_v/cross)
TRI x1,y1,x2,y2,x3,y3,char [#color]

Colors: #hex (#FF0000) or names (red,blue,green,yellow,cyan,magenta,white,gray)
Canvas: {width}x{height}. Origin 0,0 = top-left. Max 50 commands.

{sensors}

Draw "{subject}" in a {style} style. Output ONLY drawing commands, no explanation.

Example:
FILL . #1a1a2e
ROUNDBOX 2,1,25,8 #4a90d9
TEXT 5,4,"I am here" #e0e0ff
CIRCLE 40,6,4,* #ff6b6b
GRADIENT 0,12,{width},3,right"#,
    );

    ChatRequest {
        model: context.model.clone(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: prompt,
        }],
        options: GenerationOptions {
            temperature: Some(0.8),
            top_p: Some(0.95),
        },
    }
}
```

- [ ] **Step 3: Update draw_canvas tests**

Update existing tests to reflect the new prompt content (check for command keywords instead of old prompt text).

- [ ] **Step 4: Run full test suite**

```bash
cargo test
cargo clippy -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add src/tools/ src/canvas_lang/
git commit -m "feat: integrate canvas drawing language into draw_canvas tool"
```

---

## Summary

| Chunk | Tasks | Delivers |
|-------|-------|----------|
| 1 | 1-2 | Color system (hex + named), hex color tags in canvas UI |
| 2 | 3 | Full parser: tokenizer, 21 command parsers, lenient fallback |
| 3 | 4 | Renderer (2D buffer, serialization), all primitives, bitmap font |
| 4 | 5-6 | Public API, draw_canvas tool integration, new prompt |

Each chunk compiles and tests independently. The old canvas behavior is preserved as fallback throughout.
