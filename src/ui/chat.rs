use crate::app::App;
use crate::history::Role;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::Frame;

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

        if msg.text.is_empty() && !msg.complete {
            lines.push(Line::from(Span::styled(
                "▌", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK),
            )));
        }

        lines.push(Line::from(""));
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();
    let auto_bottom = if total_lines > inner_height { (total_lines - inner_height) as u16 } else { 0 };
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
