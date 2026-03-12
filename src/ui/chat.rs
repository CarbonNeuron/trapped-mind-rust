//! Chat panel rendering — scrollable conversation view with message bubbles.

use crate::app::App;
use crate::history::Role;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Renders the chat panel with all messages, auto-scrolling to the bottom
/// unless the user has manually scrolled up with PageUp.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.chat_messages {
        match msg.role {
            Role::Ai => render_ai_message(&mut lines, msg, inner_width),
            Role::User => render_user_message(&mut lines, msg, inner_width),
            Role::System => render_system_message(&mut lines, msg, inner_width),
        }
    }

    let inner_height = area.height.saturating_sub(2);
    let total_lines = lines.len() as u16;

    let auto_bottom = total_lines.saturating_sub(inner_height);
    let scroll = match app.manual_scroll {
        Some(offset) => offset.min(auto_bottom),
        None => auto_bottom,
    };

    let block = Block::bordered()
        .title(" trapped mind ")
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines).block(block).scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

/// Word-wraps text to fit within `max_width`, prepending `indent` to every line.
/// Each resulting line is exactly styled and pushed into `lines`.
fn wrap_indented(
    lines: &mut Vec<Line>,
    text: &str,
    indent: &str,
    max_width: usize,
    style: Style,
) {
    let content_width = max_width.saturating_sub(indent.len());
    if content_width == 0 {
        lines.push(Line::from(Span::styled(indent.to_string(), style)));
        return;
    }

    for logical_line in text.lines() {
        if logical_line.is_empty() {
            lines.push(Line::from(Span::styled(indent.to_string(), style)));
            continue;
        }

        let words: Vec<&str> = logical_line.split_whitespace().collect();
        if words.is_empty() {
            lines.push(Line::from(Span::styled(indent.to_string(), style)));
            continue;
        }

        let mut current_line = String::from(indent);
        let mut line_content_len = 0usize;

        for word in &words {
            let word_len = word.len();
            if line_content_len > 0 && line_content_len + 1 + word_len > content_width {
                // Flush current line and start a new one
                lines.push(Line::from(Span::styled(current_line, style)));
                current_line = String::from(indent);
                current_line.push_str(word);
                line_content_len = word_len;
            } else if line_content_len == 0 {
                current_line.push_str(word);
                line_content_len = word_len;
            } else {
                current_line.push(' ');
                current_line.push_str(word);
                line_content_len += 1 + word_len;
            }
        }

        if line_content_len > 0 {
            lines.push(Line::from(Span::styled(current_line, style)));
        }
    }
}

/// Renders an AI message with a left-aligned header and cyan text.
fn render_ai_message(
    lines: &mut Vec<Line>,
    msg: &crate::app::ChatMessage,
    inner_width: usize,
) {
    let header_style = Style::default().fg(Color::DarkGray);
    let text_style = Style::default().fg(Color::Cyan);

    // Header: ── AI HH:MM:SS ────────
    let label = if msg.timestamp.is_empty() {
        " AI ".to_string()
    } else {
        format!(" AI {} ", msg.timestamp)
    };
    let rule_len = inner_width.saturating_sub(label.len() + 2);
    let header = format!("──{}{}", label, "─".repeat(rule_len));
    lines.push(Line::from(Span::styled(header, header_style)));

    // Message body
    if msg.text.is_empty() && !msg.complete {
        lines.push(Line::from(Span::styled(
            "  ▌",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::SLOW_BLINK),
        )));
    } else {
        // Split thinking (decision model output) from tool output at "\n\n"
        let thinking_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        if let Some(sep) = msg.text.find("\n\n") {
            let thinking = &msg.text[..sep];
            let output = msg.text[sep..].trim_start_matches('\n');
            if !thinking.is_empty() {
                wrap_indented(lines, thinking, "  ", inner_width, thinking_style);
            }
            if !output.is_empty() {
                wrap_indented(lines, output, "  ", inner_width, text_style);
            }
        } else {
            wrap_indented(lines, &msg.text, "  ", inner_width, text_style);
        }
    }

    lines.push(Line::from(""));
}

/// Renders a user message with a right-aligned header and yellow text.
fn render_user_message(
    lines: &mut Vec<Line>,
    msg: &crate::app::ChatMessage,
    inner_width: usize,
) {
    let header_style = Style::default().fg(Color::DarkGray);
    let text_style = Style::default().fg(Color::Yellow);

    // Header: right-aligned ──────── YOU HH:MM:SS ──
    let label = if msg.timestamp.is_empty() {
        " YOU ".to_string()
    } else {
        format!(" YOU {} ", msg.timestamp)
    };
    let rule_len = inner_width.saturating_sub(label.len() + 2);
    let header = format!("{}{}──", "─".repeat(rule_len), label);
    lines.push(Line::from(Span::styled(header, header_style)));

    // Message body — indented
    wrap_indented(lines, &msg.text, "          ", inner_width, text_style);

    lines.push(Line::from(""));
}

/// Renders a system message as a compact dim line.
fn render_system_message(
    lines: &mut Vec<Line>,
    msg: &crate::app::ChatMessage,
    inner_width: usize,
) {
    let style = Style::default().fg(Color::DarkGray);
    wrap_indented(lines, &msg.text, "  ", inner_width, style);
    lines.push(Line::from(""));
}
