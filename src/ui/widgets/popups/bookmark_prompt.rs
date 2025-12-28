use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render the bookmark prompt popup
pub fn render_bookmark_prompt(
    frame: &mut Frame,
    input: &str,
    suggestion: Option<&str>,
    error: Option<&str>,
) {
    let area = centered_rect(50, 25, frame.area());

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the popup content
    let block = Block::default()
        .title("Add Bookmark")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout for prompt, input, suggestion, and error
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Prompt
            Constraint::Length(1), // Input
            Constraint::Length(1), // Spacing
            Constraint::Length(2), // Suggestion
            Constraint::Min(1),    // Error or hint
        ])
        .split(inner);

    // Render prompt
    let prompt = Paragraph::new("Bookmark label: ").style(Style::default().fg(Color::White));
    frame.render_widget(prompt, chunks[0]);

    // Render input
    let input_text = Paragraph::new(input).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input_text, chunks[1]);

    // Render suggestion if available
    if let Some(sug) = suggestion {
        let suggestion_text = vec![
            Line::from(Span::styled(
                "Suggestion:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(sug, Style::default().fg(Color::Green))),
        ];
        let suggestion_paragraph = Paragraph::new(suggestion_text).wrap(Wrap { trim: true });
        frame.render_widget(suggestion_paragraph, chunks[3]);
    }

    // Render error or hint
    if let Some(err) = error {
        let error_text = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        frame.render_widget(error_text, chunks[4]);
    } else if input.is_empty() && suggestion.is_none() {
        let hint = Paragraph::new("Enter a label for this bookmark")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[4]);
    }
}

/// Create a centered rect using a percentage of the available space
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
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
        .split(popup_layout[1])[1]
}
