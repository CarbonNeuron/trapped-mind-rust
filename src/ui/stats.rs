//! System stats panel — color-coded gauges for CPU, temperature, RAM, battery,
//! fan speed, uptime, and network interfaces. Respects stats visibility config.

use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

/// Renders a fixed-width progress bar using block characters.
fn progress_bar(value: f32, max: f32, width: usize) -> String {
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * width as f32) as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Returns a green/yellow/red color based on CPU percentage.
fn cpu_color(pct: f32) -> Color {
    if pct > 80.0 { Color::Red } else if pct > 50.0 { Color::Yellow } else { Color::Green }
}

/// Returns a green/yellow/red color based on temperature in Celsius.
fn temp_color(temp: f32) -> Color {
    if temp > 70.0 { Color::Red } else if temp > 55.0 { Color::Yellow } else { Color::Green }
}

/// Returns a red/yellow/green color based on battery percentage (inverted scale).
fn battery_color(pct: f32) -> Color {
    if pct < 20.0 { Color::Red } else if pct < 50.0 { Color::Yellow } else { Color::Green }
}

/// Renders the system stats panel with live gauges and network info.
/// Only displays stats that are enabled in the config.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let info = &app.system_info;
    let vis = &app.config.stats;

    let mut lines: Vec<Line> = Vec::new();

    if vis.cpu {
        lines.push(Line::from(vec![
            Span::styled("CPU:  ", Style::default().fg(Color::White)),
            Span::styled(format!("{:>3.0}% ", info.cpu_percent), Style::default().fg(cpu_color(info.cpu_percent))),
            Span::styled(progress_bar(info.cpu_percent, 100.0, 8), Style::default().fg(cpu_color(info.cpu_percent))),
        ]));
    }

    if vis.temperature {
        lines.push(Line::from(vec![
            Span::styled("TEMP: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.0}°C", info.temp_celsius), Style::default().fg(temp_color(info.temp_celsius))),
        ]));
    }

    if vis.ram {
        lines.push(Line::from(vec![
            Span::styled("RAM:  ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.1}G/{:.1}G ", info.ram_used_gb(), info.ram_total_gb()), Style::default().fg(Color::Cyan)),
            Span::styled(progress_bar(info.ram_used_bytes as f32, info.ram_total_bytes as f32, 8), Style::default().fg(Color::Cyan)),
        ]));
    }

    if vis.battery {
        lines.push(Line::from(vec![
            Span::styled("BAT:  ", Style::default().fg(Color::White)),
            Span::styled(format!("{:>3.0}% ", info.battery_percent), Style::default().fg(battery_color(info.battery_percent))),
            Span::styled(progress_bar(info.battery_percent, 100.0, 8), Style::default().fg(battery_color(info.battery_percent))),
        ]));
        lines.push(Line::from(vec![
            Span::styled("PWR:  ", Style::default().fg(Color::White)),
            Span::styled(info.power_status.clone(), Style::default().fg(Color::White)),
        ]));
    }

    if vis.fan {
        lines.push(Line::from(vec![
            Span::styled("FAN:  ", Style::default().fg(Color::White)),
            Span::styled(format!("{} RPM", info.fan_rpm), Style::default().fg(if info.fan_rpm > 4000 { Color::Red } else { Color::Gray })),
        ]));
    }

    if vis.uptime {
        lines.push(Line::from(vec![
            Span::styled("UP:   ", Style::default().fg(Color::White)),
            Span::styled(info.uptime_formatted(), Style::default().fg(Color::White)),
        ]));
    }

    if vis.network && !info.networks.is_empty() {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled("NET:", Style::default().fg(Color::White))));
        for net in &info.networks {
            lines.push(Line::from(Span::styled(
                format!(" {}: {}", net.name, net.ip), Style::default().fg(Color::Green),
            )));
        }
    }

    let block = Block::bordered()
        .title(" SYSTEM STATS ")
        .style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
