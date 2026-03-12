//! UI rendering — splits the terminal into four panels and delegates to submodules.
//!
//! Layout: chat (left 70%), pet face (top-right 30%), system stats
//! (bottom-right 30%), and input bar (full-width bottom).
//! When in config mode, an overlay is drawn on top of the normal UI.

pub mod chat;
pub mod config;
pub mod input;
pub mod pet;
pub mod stats;

use crate::app::{App, AppMode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

/// Renders the full application UI into the given terminal frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let main_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(outer[0]);

    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_area[1]);

    chat::render(frame, main_area[0], app);
    pet::render(frame, right_panel[0], app);
    stats::render(frame, right_panel[1], app);
    input::render(frame, outer[1], app);

    // Config overlay on top of everything
    if app.mode == AppMode::Config {
        config::render(frame, app);
    }
}
