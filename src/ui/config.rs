//! Config menu panel — full-screen overlay for editing settings.

use crate::app::{App, ConfigField};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

/// Renders the config menu as a centered overlay.
pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 60, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .split(area);

    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, field) in ConfigField::ALL.iter().enumerate() {
        let selected = i == app.config_selected;
        let label = field.label();
        let value = if app.config_editing && selected {
            format!("{}|", &app.config_edit_buffer)
        } else {
            app.config_field_value(*field)
        };

        let arrow = if selected { "> " } else { "  " };
        let style = if selected {
            if app.config_editing {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default().fg(Color::White)
        };

        // Truncate long values (like system prompt) for display
        let display_value = if value.len() > 40 {
            format!("{}...", &value[..37])
        } else {
            value
        };

        lines.push(Line::from(Span::styled(
            format!("{}{:<22} {}", arrow, label, display_value),
            style,
        )));
    }

    lines.push(Line::from(""));

    let block = Block::bordered()
        .title(" Configuration ")
        .style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, outer[0]);

    // Hint bar
    let hint = if app.config_editing {
        " Type value, Enter to confirm, Esc to cancel "
    } else {
        " ↑↓ Navigate  Enter Edit/Toggle  Esc Close "
    };
    let hint_block = Block::bordered()
        .style(Style::default().fg(Color::DarkGray));
    let hint_para = Paragraph::new(Line::from(Span::styled(
        hint, Style::default().fg(Color::DarkGray),
    ))).block(hint_block);
    frame.render_widget(hint_para, outer[1]);
}

/// Returns a centered `Rect` that's `percent_x` wide and `percent_y` tall.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
