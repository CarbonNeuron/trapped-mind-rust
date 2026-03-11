use crate::app::App;
use crate::pet_states::PetMood;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mood = PetMood::from_state(&app.system_info, app.is_generating, app.is_user_typing);
    let frames = mood.frames();
    let frame_index = app.pet_frame_index % frames.len();
    let current_frame = frames[frame_index];
    let color = mood.color();

    let lines: Vec<Line> = current_frame
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(color))))
        .collect();

    let block = Block::bordered()
        .title(format!(" {:?} ", mood))
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines).block(block).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
