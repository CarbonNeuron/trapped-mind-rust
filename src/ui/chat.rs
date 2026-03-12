//! Chat panel rendering — scrollable conversation view with message bubbles.

use crate::app::App;
use crate::history::Role;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
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
            Role::System => render_system_message(&mut lines, msg),
        }
    }

    let inner_height = area.height.saturating_sub(2);

    // Count visual lines after word-wrapping (ceiling division)
    let total_wrapped: u16 = lines
        .iter()
        .map(|line| {
            let width = line.width();
            if width == 0 || inner_width == 0 {
                1u16
            } else {
                width.div_ceil(inner_width) as u16
            }
        })
        .sum();

    let auto_bottom = total_wrapped.saturating_sub(inner_height);
    let scroll = match app.manual_scroll {
        Some(offset) => offset.min(auto_bottom),
        None => auto_bottom,
    };

    let block = Block::bordered()
        .title(" trapped mind ")
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

/// Renders an AI message with a left-aligned header and cyan text.
fn render_ai_message(
    lines: &mut Vec<Line>,
    msg: &crate::app::ChatMessage,
    inner_width: usize,
) {
    let header_style = Style::default().fg(Color::DarkGray);
    let text_style = Style::default().fg(Color::Cyan);

    // Header: ── AI · HH:MM:SS ────────
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
        for line_text in msg.text.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", line_text),
                text_style,
            )));
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

    // Header: right-aligned ──────── YOU · HH:MM:SS ──
    let label = if msg.timestamp.is_empty() {
        " YOU ".to_string()
    } else {
        format!(" YOU {} ", msg.timestamp)
    };
    let rule_len = inner_width.saturating_sub(label.len() + 2);
    let header = format!("{}{}──", "─".repeat(rule_len), label);
    lines.push(Line::from(Span::styled(header, header_style)));

    // Message body — right-indented with a few spaces
    let indent = "          ";
    for line_text in msg.text.lines() {
        lines.push(Line::from(Span::styled(
            format!("{}{}", indent, line_text),
            text_style,
        )));
    }

    lines.push(Line::from(""));
}

/// Renders a system message as a compact dim line.
fn render_system_message(lines: &mut Vec<Line>, msg: &crate::app::ChatMessage) {
    let style = Style::default().fg(Color::DarkGray);
    for line_text in msg.text.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", line_text),
            style,
        )));
    }
    lines.push(Line::from(""));
}
