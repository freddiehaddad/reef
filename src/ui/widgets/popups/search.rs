use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render the search popup
pub fn render_search_popup(frame: &mut Frame, input: &str, error: Option<&str>) {
    let area = centered_rect(50, 20, frame.area());

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the popup content
    let block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout for search input and error/hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Prompt
            Constraint::Length(1), // Input
            Constraint::Min(1),    // Error or hint
        ])
        .split(inner);

    // Render prompt
    let prompt = Paragraph::new("Search: ").style(Style::default().fg(Color::White));
    frame.render_widget(prompt, chunks[0]);

    // Render input
    let input_text = Paragraph::new(input).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input_text, chunks[1]);

    // Render error or hint
    if let Some(err) = error {
        let error_text = Paragraph::new(err)
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        frame.render_widget(error_text, chunks[2]);
    } else if input.is_empty() {
        let hint = Paragraph::new("Enter search query (regex supported)")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[2]);
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
