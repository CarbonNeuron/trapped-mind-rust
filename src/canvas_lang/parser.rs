//! Script parser for the canvas drawing language.
//!
//! Converts a text script into a sequence of drawing commands.

use crate::canvas_lang::color::{parse_color, CanvasColor};

/// Maximum number of commands parsed from a single script.
const MAX_COMMANDS: usize = 50;

/// Direction for gradient fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientDir {
    Left,
    Right,
    Up,
    Down,
}

/// Pattern types for patterned fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Checker,
    Dots,
    StripesH,
    StripesV,
    Cross,
}

/// A single drawing command produced by the parser.
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

/// Tokenizes a single line: splits on whitespace and commas, but preserves
/// quoted strings (double quotes) as single tokens (quotes stripped).
pub fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' || c == ',' {
            chars.next();
        } else if c == '"' {
            chars.next(); // consume opening quote
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '"' {
                    chars.next(); // consume closing quote
                    break;
                }
                s.push(ch);
                chars.next();
            }
            tokens.push(s);
        } else {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == ' ' || ch == '\t' || ch == ',' || ch == '"' {
                    break;
                }
                s.push(ch);
                chars.next();
            }
            tokens.push(s);
        }
    }

    tokens
}

/// Helper: tries to parse the last token as a color. If it succeeds, returns
/// `(remaining_tokens, Some(color))`. Otherwise returns `(all_tokens, None)`.
fn split_trailing_color(tokens: &[String]) -> (&[String], Option<CanvasColor>) {
    if let Some(last) = tokens.last() {
        if let Some(color) = parse_color(last) {
            return (&tokens[..tokens.len() - 1], Some(color));
        }
    }
    (tokens, None)
}

fn parse_i32(s: &str) -> Option<i32> {
    s.parse::<i32>().ok()
}

fn parse_u32(s: &str) -> Option<u32> {
    s.parse::<u32>().ok()
}

fn parse_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_none() {
        Some(c)
    } else {
        None
    }
}

fn parse_gradient_dir(s: &str) -> Option<GradientDir> {
    match s.to_lowercase().as_str() {
        "left" => Some(GradientDir::Left),
        "right" => Some(GradientDir::Right),
        "up" => Some(GradientDir::Up),
        "down" => Some(GradientDir::Down),
        _ => None,
    }
}

fn parse_pattern_type(s: &str) -> Option<PatternType> {
    match s.to_lowercase().as_str() {
        "checker" => Some(PatternType::Checker),
        "dots" => Some(PatternType::Dots),
        "stripesh" => Some(PatternType::StripesH),
        "stripesv" => Some(PatternType::StripesV),
        "cross" => Some(PatternType::Cross),
        _ => None,
    }
}

// Individual command parsers. Each takes the argument tokens (keyword already removed).

fn parse_clear(args: &[String]) -> Option<DrawCommand> {
    if args.is_empty() {
        Some(DrawCommand::Clear)
    } else {
        None
    }
}

fn parse_fill(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 1 {
        return None;
    }
    let ch = parse_char(&rest[0])?;
    Some(DrawCommand::Fill { ch, color })
}

fn parse_rect(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 5 {
        return None;
    }
    Some(DrawCommand::Rect {
        x: parse_i32(&rest[0])?,
        y: parse_i32(&rest[1])?,
        w: parse_u32(&rest[2])?,
        h: parse_u32(&rest[3])?,
        ch: parse_char(&rest[4])?,
        color,
    })
}

fn parse_outline(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 5 {
        return None;
    }
    Some(DrawCommand::Outline {
        x: parse_i32(&rest[0])?,
        y: parse_i32(&rest[1])?,
        w: parse_u32(&rest[2])?,
        h: parse_u32(&rest[3])?,
        ch: parse_char(&rest[4])?,
        color,
    })
}

fn parse_roundbox(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::RoundBox {
        x: parse_i32(&rest[0])?,
        y: parse_i32(&rest[1])?,
        w: parse_u32(&rest[2])?,
        h: parse_u32(&rest[3])?,
        color,
    })
}

fn parse_frame(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::Frame {
        x: parse_i32(&rest[0])?,
        y: parse_i32(&rest[1])?,
        w: parse_u32(&rest[2])?,
        h: parse_u32(&rest[3])?,
        color,
    })
}

fn parse_circle(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::Circle {
        cx: parse_i32(&rest[0])?,
        cy: parse_i32(&rest[1])?,
        r: parse_u32(&rest[2])?,
        ch: parse_char(&rest[3])?,
        color,
    })
}

fn parse_ring(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::Ring {
        cx: parse_i32(&rest[0])?,
        cy: parse_i32(&rest[1])?,
        r: parse_u32(&rest[2])?,
        ch: parse_char(&rest[3])?,
        color,
    })
}

fn parse_ellipse(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 5 {
        return None;
    }
    Some(DrawCommand::Ellipse {
        cx: parse_i32(&rest[0])?,
        cy: parse_i32(&rest[1])?,
        rx: parse_u32(&rest[2])?,
        ry: parse_u32(&rest[3])?,
        ch: parse_char(&rest[4])?,
        color,
    })
}

fn parse_hline(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::HLine {
        y: parse_i32(&rest[0])?,
        x1: parse_i32(&rest[1])?,
        x2: parse_i32(&rest[2])?,
        ch: parse_char(&rest[3])?,
        color,
    })
}

fn parse_vline(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::VLine {
        x: parse_i32(&rest[0])?,
        y1: parse_i32(&rest[1])?,
        y2: parse_i32(&rest[2])?,
        ch: parse_char(&rest[3])?,
        color,
    })
}

fn parse_line(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 5 {
        return None;
    }
    Some(DrawCommand::Line {
        x1: parse_i32(&rest[0])?,
        y1: parse_i32(&rest[1])?,
        x2: parse_i32(&rest[2])?,
        y2: parse_i32(&rest[3])?,
        ch: parse_char(&rest[4])?,
        color,
    })
}

fn parse_arrow(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::Arrow {
        x1: parse_i32(&rest[0])?,
        y1: parse_i32(&rest[1])?,
        x2: parse_i32(&rest[2])?,
        y2: parse_i32(&rest[3])?,
        color,
    })
}

fn parse_boxline(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 4 {
        return None;
    }
    Some(DrawCommand::BoxLine {
        x1: parse_i32(&rest[0])?,
        y1: parse_i32(&rest[1])?,
        x2: parse_i32(&rest[2])?,
        y2: parse_i32(&rest[3])?,
        color,
    })
}

fn parse_text_cmd(args: &[String]) -> Option<DrawCommand> {
    if args.len() < 3 {
        return None;
    }
    let x = parse_i32(&args[0])?;
    let y = parse_i32(&args[1])?;
    let text_tokens = &args[2..];
    let (rest, color) = split_trailing_color(text_tokens);
    let text = if rest.is_empty() {
        // The "color" was actually the only text token, so there's no color.
        text_tokens.join(" ")
    } else {
        rest.join(" ")
    };
    let actual_color = if rest.is_empty() { None } else { color };
    if text.is_empty() {
        return None;
    }
    Some(DrawCommand::Text { x, y, text, color: actual_color })
}

fn parse_bigtext_cmd(args: &[String]) -> Option<DrawCommand> {
    if args.len() < 3 {
        return None;
    }
    let x = parse_i32(&args[0])?;
    let y = parse_i32(&args[1])?;
    let text_tokens = &args[2..];
    let (rest, color) = split_trailing_color(text_tokens);
    let text = if rest.is_empty() {
        text_tokens.join(" ")
    } else {
        rest.join(" ")
    };
    let actual_color = if rest.is_empty() { None } else { color };
    if text.is_empty() {
        return None;
    }
    Some(DrawCommand::BigText { x, y, text, color: actual_color })
}

fn parse_gradient(args: &[String]) -> Option<DrawCommand> {
    if args.len() != 5 {
        return None;
    }
    Some(DrawCommand::Gradient {
        x: parse_i32(&args[0])?,
        y: parse_i32(&args[1])?,
        w: parse_u32(&args[2])?,
        h: parse_u32(&args[3])?,
        direction: parse_gradient_dir(&args[4])?,
    })
}

fn parse_pattern(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 5 {
        return None;
    }
    Some(DrawCommand::Pattern {
        x: parse_i32(&rest[0])?,
        y: parse_i32(&rest[1])?,
        w: parse_u32(&rest[2])?,
        h: parse_u32(&rest[3])?,
        pattern: parse_pattern_type(&rest[4])?,
        color,
    })
}

fn parse_tri(args: &[String]) -> Option<DrawCommand> {
    let (rest, color) = split_trailing_color(args);
    if rest.len() != 7 {
        return None;
    }
    Some(DrawCommand::Tri {
        x1: parse_i32(&rest[0])?,
        y1: parse_i32(&rest[1])?,
        x2: parse_i32(&rest[2])?,
        y2: parse_i32(&rest[3])?,
        x3: parse_i32(&rest[4])?,
        y3: parse_i32(&rest[5])?,
        ch: parse_char(&rest[6])?,
        color,
    })
}

fn parse_line_to_command(tokens: &[String]) -> Option<DrawCommand> {
    if tokens.is_empty() {
        return None;
    }
    let keyword = tokens[0].to_lowercase();
    let args = &tokens[1..];
    match keyword.as_str() {
        "clear" => parse_clear(args),
        "fill" => parse_fill(args),
        "rect" => parse_rect(args),
        "outline" => parse_outline(args),
        "roundbox" => parse_roundbox(args),
        "frame" => parse_frame(args),
        "circle" => parse_circle(args),
        "ring" => parse_ring(args),
        "ellipse" => parse_ellipse(args),
        "hline" => parse_hline(args),
        "vline" => parse_vline(args),
        "line" => parse_line(args),
        "arrow" => parse_arrow(args),
        "boxline" => parse_boxline(args),
        "text" => parse_text_cmd(args),
        "bigtext" => parse_bigtext_cmd(args),
        "gradient" => parse_gradient(args),
        "pattern" => parse_pattern(args),
        "tri" => parse_tri(args),
        _ => None,
    }
}

/// Parses a canvas-lang script into a list of draw commands.
///
/// Reads the input line by line. Unrecognized or malformed lines are silently
/// skipped. Parsing stops after [`MAX_COMMANDS`] valid commands.
pub fn parse_script(input: &str) -> Vec<DrawCommand> {
    let mut commands = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let tokens = tokenize(trimmed);
        if let Some(cmd) = parse_line_to_command(&tokens) {
            commands.push(cmd);
            if commands.len() >= MAX_COMMANDS {
                break;
            }
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_lang::color::CanvasColor;

    // ---- tokenize tests ----

    #[test]
    fn tokenize_simple() {
        let tokens = tokenize("RECT 10 20 30 40 #");
        assert_eq!(tokens, vec!["RECT", "10", "20", "30", "40", "#"]);
    }

    #[test]
    fn tokenize_commas() {
        let tokens = tokenize("RECT 10,20,30,40 #");
        assert_eq!(tokens, vec!["RECT", "10", "20", "30", "40", "#"]);
    }

    #[test]
    fn tokenize_quoted_string() {
        let tokens = tokenize("TEXT 5 3 \"Hello World\"");
        assert_eq!(tokens, vec!["TEXT", "5", "3", "Hello World"]);
    }

    #[test]
    fn tokenize_mixed() {
        let tokens = tokenize("TEXT 5,3 \"hello world\" red");
        assert_eq!(tokens, vec!["TEXT", "5", "3", "hello world", "red"]);
    }

    // ---- parse_script tests ----

    #[test]
    fn parse_clear() {
        let cmds = parse_script("CLEAR");
        assert_eq!(cmds, vec![DrawCommand::Clear]);
    }

    #[test]
    fn parse_fill_no_color() {
        let cmds = parse_script("FILL .");
        assert_eq!(cmds, vec![DrawCommand::Fill { ch: '.', color: None }]);
    }

    #[test]
    fn parse_fill_with_color() {
        let cmds = parse_script("FILL * red");
        assert_eq!(cmds, vec![DrawCommand::Fill { ch: '*', color: Some(CanvasColor::new(205, 0, 0)) }]);
    }

    #[test]
    fn parse_rect_basic() {
        let cmds = parse_script("RECT 0 0 10 5 #");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Rect { x, y, w, h, ch, color } => {
                assert_eq!((*x, *y, *w, *h, *ch), (0, 0, 10, 5, '#'));
                assert_eq!(*color, None);
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_rect_with_color() {
        let cmds = parse_script("RECT 0 0 10 5 # blue");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Rect { color, .. } => {
                assert!(color.is_some());
            }
            _ => panic!("expected Rect"),
        }
    }

    #[test]
    fn parse_text_with_quotes() {
        let cmds = parse_script("TEXT 5 3 \"Hello World\"");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Text { x, y, text, color } => {
                assert_eq!((*x, *y), (5, 3));
                assert_eq!(text, "Hello World");
                assert_eq!(*color, None);
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_text_unquoted() {
        let cmds = parse_script("TEXT 1 2 hi there");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Text { text, color, .. } => {
                assert_eq!(text, "hi there");
                assert_eq!(*color, None);
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_text_with_color() {
        let cmds = parse_script("TEXT 1 2 hello red");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Text { text, color, .. } => {
                assert_eq!(text, "hello");
                assert!(color.is_some());
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_gradient() {
        let cmds = parse_script("GRADIENT 0 0 40 20 right");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Gradient { x, y, w, h, direction } => {
                assert_eq!((*x, *y, *w, *h), (0, 0, 40, 20));
                assert_eq!(*direction, GradientDir::Right);
            }
            _ => panic!("expected Gradient"),
        }
    }

    #[test]
    fn parse_pattern() {
        let cmds = parse_script("PATTERN 0 0 10 10 checker red");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Pattern { pattern, color, .. } => {
                assert_eq!(*pattern, PatternType::Checker);
                assert!(color.is_some());
            }
            _ => panic!("expected Pattern"),
        }
    }

    #[test]
    fn parse_circle() {
        let cmds = parse_script("CIRCLE 20 10 8 *");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Circle { cx, cy, r, ch, color } => {
                assert_eq!((*cx, *cy, *r, *ch), (20, 10, 8, '*'));
                assert_eq!(*color, None);
            }
            _ => panic!("expected Circle"),
        }
    }

    #[test]
    fn parse_tri() {
        let cmds = parse_script("TRI 0 10 5 0 10 10 ^");
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            DrawCommand::Tri { x1, y1, x2, y2, x3, y3, ch, color } => {
                assert_eq!((*x1, *y1, *x2, *y2, *x3, *y3, *ch), (0, 10, 5, 0, 10, 10, '^'));
                assert_eq!(*color, None);
            }
            _ => panic!("expected Tri"),
        }
    }

    #[test]
    fn ignores_garbage_lines() {
        let cmds = parse_script("this is not a command\nCLEAR\nblah blah");
        assert_eq!(cmds, vec![DrawCommand::Clear]);
    }

    #[test]
    fn case_insensitive() {
        let cmds = parse_script("clear");
        assert_eq!(cmds, vec![DrawCommand::Clear]);

        let cmds2 = parse_script("Clear");
        assert_eq!(cmds2, vec![DrawCommand::Clear]);

        let cmds3 = parse_script("fill . RED");
        assert_eq!(cmds3.len(), 1);
        match &cmds3[0] {
            DrawCommand::Fill { ch, color } => {
                assert_eq!(*ch, '.');
                assert!(color.is_some());
            }
            _ => panic!("expected Fill"),
        }
    }

    #[test]
    fn max_commands_limit() {
        let script: String = (0..100).map(|_| "CLEAR\n").collect();
        let cmds = parse_script(&script);
        assert_eq!(cmds.len(), MAX_COMMANDS);
    }

    #[test]
    fn multi_command_script() {
        let script = "CLEAR\nFILL . red\nRECT 0 0 10 5 # blue\nTEXT 1 1 hello";
        let cmds = parse_script(script);
        assert_eq!(cmds.len(), 4);
        assert_eq!(cmds[0], DrawCommand::Clear);
        assert!(matches!(cmds[1], DrawCommand::Fill { .. }));
        assert!(matches!(cmds[2], DrawCommand::Rect { .. }));
        assert!(matches!(cmds[3], DrawCommand::Text { .. }));
    }

    #[test]
    fn skips_comments() {
        let cmds = parse_script("# comment\n// another comment\nCLEAR");
        assert_eq!(cmds, vec![DrawCommand::Clear]);
    }

    #[test]
    fn parse_hline() {
        let cmds = parse_script("HLINE 5 0 20 -");
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], DrawCommand::HLine { y: 5, x1: 0, x2: 20, .. }));
    }

    #[test]
    fn parse_arrow() {
        let cmds = parse_script("ARROW 0 0 10 10 green");
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], DrawCommand::Arrow { x1: 0, y1: 0, x2: 10, y2: 10, .. }));
    }
}
