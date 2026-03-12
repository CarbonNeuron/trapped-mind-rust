//! Chat panel rendering — scrollable conversation view with colored roles.

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
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.chat_messages {
        let (prefix, style) = match msg.role {
            Role::Ai => ("", Style::default().fg(Color::Cyan)),
            Role::User => ("> USER: ", Style::default().fg(Color::Yellow)),
            Role::System => ("", Style::default().fg(Color::DarkGray)),
        };

        for (i, line_text) in msg.text.lines().enumerate() {
            let text = if i == 0 && !prefix.is_empty() {
                format!("{}{}", prefix, line_text)
            } else {
                line_text.to_string()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }

        // Show a blinking cursor for in-progress AI messages
        if msg.text.is_empty() && !msg.complete {
            lines.push(Line::from(Span::styled(
                "▌", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK),
            )));
        }

        lines.push(Line::from(""));
    }

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2);

    // Count visual lines after word-wrapping (ceiling division)
    let total_wrapped: u16 = lines.iter().map(|line| {
        let width = line.width();
        if width == 0 || inner_width == 0 {
            1u16
        } else {
            width.div_ceil(inner_width) as u16
        }
    }).sum();

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
