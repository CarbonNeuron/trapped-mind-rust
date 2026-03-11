//! Input bar panel — text input with visible cursor and placeholder text.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Renders the input bar with a blinking block cursor.
///
/// Shows placeholder text when the buffer is empty, otherwise renders the
/// input text with a highlighted cursor character.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let display_text = if app.input_buffer.is_empty() {
        Line::from(Span::styled(
            "Type a message... (/help for commands)",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        let before_cursor = &app.input_buffer[..app.input_cursor];
        let after_cursor = &app.input_buffer[app.input_cursor..];

        let (cursor_display, rest) = if after_cursor.is_empty() {
            (" ".to_string(), "")
        } else {
            // Extract the first character after the cursor for highlighting
            let first_char_end = after_cursor
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(after_cursor.len());
            (after_cursor[..first_char_end].to_string(), &after_cursor[first_char_end..])
        };

        Line::from(vec![
            Span::styled(before_cursor.to_string(), Style::default().fg(Color::White)),
            Span::styled(cursor_display, Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
            Span::styled(rest.to_string(), Style::default().fg(Color::White)),
        ])
    };

    let block = Block::bordered()
        .title(" > ")
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(display_text).block(block);
    frame.render_widget(paragraph, area);
}
