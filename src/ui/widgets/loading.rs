//! Loading spinner and progress indicator widgets

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Loading indicator with optional progress
pub struct LoadingWidget {
    message: String,
    progress: Option<(usize, usize)>, // (current, total)
}

impl LoadingWidget {
    /// Create a new loading widget
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            progress: None,
        }
    }

    /// Set progress (current, total)
    pub fn progress(mut self, current: usize, total: usize) -> Self {
        self.progress = Some((current, total));
        self
    }

    /// Render the loading widget as a centered popup
    pub fn render(&self, f: &mut Frame, area: Rect) {
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
                        "⏳",
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
            // Just loading message
            vec![Line::from(vec![
                Span::styled(
                    "⏳",
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

        // Center the widget - width based on content
        // Progress bar (40) + borders (2) + padding (4) = 46
        let width = 46;
        let height = 8;
        let area = centered_rect_fixed(width, height, area);
        f.render_widget(paragraph, area);
    }
}

/// Helper function to create a centered rect with fixed dimensions
fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
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
            Constraint::Length((r.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Length((r.width.saturating_sub(width)) / 2),
        ])
        .split(popup_layout[1])[1]
}
