//! Loading spinner and progress indicator widgets

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Instant;

/// Spinner animation styles
#[derive(Debug, Clone, Copy)]
pub enum SpinnerStyle {
    /// Braille dots: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
    Dots,
}

impl SpinnerStyle {
    fn frames(&self) -> &'static [&'static str] {
        match self {
            Self::Dots => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        }
    }

    fn current_frame(&self, start_time: Instant) -> &'static str {
        let frames = self.frames();
        let elapsed_ms = start_time.elapsed().as_millis();
        let idx = (elapsed_ms / 80) as usize % frames.len();
        frames[idx]
    }
}

/// Loading indicator with optional progress
pub struct LoadingWidget {
    style: SpinnerStyle,
    message: String,
    progress: Option<(usize, usize)>, // (current, total)
    start_time: Instant,
}

impl LoadingWidget {
    /// Create a new loading widget
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            style: SpinnerStyle::Dots,
            message: message.into(),
            progress: None,
            start_time: Instant::now(),
        }
    }

    /// Set progress (current, total)
    pub fn progress(mut self, current: usize, total: usize) -> Self {
        self.progress = Some((current, total));
        self
    }

    /// Render the loading widget as a centered popup
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let spinner_char = self.style.current_frame(self.start_time);

        let lines = if let Some((current, total)) = self.progress {
            // With progress bar
            let percentage = if total > 0 {
                (current * 100) / total
            } else {
                0
            };

            let bar_width = 40;
            let filled = (bar_width * current) / total.max(1);
            let empty = bar_width - filled;

            let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            vec![
                Line::from(vec![
                    Span::styled(
                        spinner_char,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(&self.message, Style::default().fg(Color::White)),
                ]),
                Line::raw(""),
                Line::from(vec![Span::styled(
                    progress_bar,
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(vec![Span::styled(
                    format!("{}/{} ({}%)", current, total, percentage),
                    Style::default().fg(Color::Gray),
                )]),
            ]
        } else {
            // Just spinner
            vec![Line::from(vec![
                Span::styled(
                    spinner_char,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(&self.message, Style::default().fg(Color::White)),
            ])]
        };

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Loading ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        );

        // Center the widget
        let area = centered_rect(60, 8, area);
        f.render_widget(paragraph, area);
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
