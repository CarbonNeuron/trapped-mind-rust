//! Canvas panel — AI-generated ASCII art that replaces the static pet face.

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

    let color = if app.canvas_generating {
        Color::DarkGray
    } else {
        Color::Cyan
    };

    let lines: Vec<Line> = if app.canvas_lines.is_empty() {
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
            .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(color))))
            .collect()
    };

    let title = if app.canvas_generating {
        " canvas (generating...) "
    } else {
        " canvas "
    };

    let block = Block::bordered()
        .title(title)
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
