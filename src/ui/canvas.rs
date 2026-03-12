//! Canvas panel — AI-generated ASCII art that replaces the static pet face.
//!
//! Supports inline color tags: `{red}text{/}` etc. Tags are parsed into
//! colored ratatui spans. Lines are truncated to the panel width to prevent
//! wrapping.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Renders the canvas panel, updating stored dimensions on the App
/// so canvas generation knows the target size.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);

    // Store dimensions so canvas generation can target them
    app.canvas_width = inner_width;
    app.canvas_height = inner_height;

    let default_color = if app.canvas_generating {
        Color::White
    } else {
        Color::Cyan
    };

    let lines: Vec<Line> = if app.canvas_lines.is_empty() && !app.canvas_generating {
        // Show a placeholder when no canvas has been generated yet
        let mut placeholder = Vec::new();
        let msg = "awaiting vision...";
        let pad_top = (inner_height as usize).saturating_sub(1) / 2;
        for _ in 0..pad_top {
            placeholder.push(Line::from(""));
        }
        let pad_left = (inner_width as usize).saturating_sub(msg.len()) / 2;
        placeholder.push(Line::from(Span::styled(
            format!("{}{}", " ".repeat(pad_left), msg),
            Style::default().fg(Color::DarkGray),
        )));
        placeholder
    } else {
        app.canvas_lines
            .iter()
            .map(|line| parse_colored_line(line, default_color, inner_width as usize))
            .collect()
    };

    let title = if app.canvas_generating {
        " canvas ~ "
    } else {
        " canvas "
    };

    let block = Block::bordered()
        .title(title)
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Parses a line with inline color tags into a sequence of colored spans.
///
/// Supported tags: `{red}`, `{green}`, `{blue}`, `{yellow}`, `{cyan}`,
/// `{magenta}`, `{white}`, `{gray}`, `{/}` (reset to default).
///
/// Visible characters are counted and the line is truncated at `max_width`
/// to prevent wrapping.
fn parse_colored_line(line: &str, default_color: Color, max_width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_color = default_color;
    let mut buf = String::new();
    let mut visible_count = 0usize;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if visible_count >= max_width {
            break;
        }

        if ch == '{' {
            // Try to parse a color tag
            let mut tag = String::new();
            let mut found_close = false;
            // Collect up to 10 chars looking for '}'
            let mut lookahead: Vec<char> = Vec::new();
            for _ in 0..10 {
                if let Some(&next) = chars.peek() {
                    lookahead.push(next);
                    chars.next();
                    if next == '}' {
                        found_close = true;
                        break;
                    }
                    tag.push(next);
                } else {
                    break;
                }
            }

            if found_close {
                if let Some(color) = parse_color_tag(&tag, default_color) {
                    // Flush buffer with current color
                    if !buf.is_empty() {
                        spans.push(Span::styled(
                            buf.clone(),
                            Style::default().fg(current_color),
                        ));
                        buf.clear();
                    }
                    current_color = color;
                    continue;
                }
            }

            // Not a valid tag — emit '{' and the lookahead as literal chars
            if visible_count < max_width {
                buf.push('{');
                visible_count += 1;
            }
            for lc in lookahead {
                if visible_count >= max_width {
                    break;
                }
                buf.push(lc);
                visible_count += 1;
            }
        } else {
            buf.push(ch);
            visible_count += 1;
        }
    }

    // Flush remaining buffer
    if !buf.is_empty() {
        spans.push(Span::styled(buf, Style::default().fg(current_color)));
    }

    if spans.is_empty() {
        Line::from("")
    } else {
        Line::from(spans)
    }
}

/// Maps a tag name (without braces) to a color, or `None` if unrecognized.
fn parse_color_tag(tag: &str, default_color: Color) -> Option<Color> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_line() {
        let line = parse_colored_line("hello world", Color::Cyan, 80);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "hello world");
    }

    #[test]
    fn test_colored_spans() {
        let line = parse_colored_line("{red}fire{/} and {blue}ice{/}", Color::Cyan, 80);
        assert!(line.spans.len() >= 3);
        assert_eq!(line.spans[0].content, "fire");
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }

    #[test]
    fn test_truncation() {
        let line = parse_colored_line("abcdefghij", Color::Cyan, 5);
        let total: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(total, "abcde");
    }

    #[test]
    fn test_color_tags_dont_count_as_visible() {
        let line = parse_colored_line("{red}abc{/}de", Color::Cyan, 5);
        let total: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(total, "abcde");
    }

    #[test]
    fn test_invalid_tag_rendered_literally() {
        let line = parse_colored_line("{notacolor}hi", Color::Cyan, 80);
        let total: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(total.contains("{notacolor}"));
    }

    #[test]
    fn test_unclosed_brace() {
        let line = parse_colored_line("hello { world", Color::Cyan, 80);
        let total: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert!(total.contains("{"));
    }
}
