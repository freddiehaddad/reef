use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Render the search popup
pub fn render_search_popup(frame: &mut Frame, input: &str, error: Option<&str>) {
    // Calculate popup width (50% of screen width)
    let popup_width = (frame.area().width as f32 * 0.5) as u16;
    // Height for one line of input plus borders
    let popup_height = 3;

    let popup_x = (frame.area().width.saturating_sub(popup_width)) / 2;
    let popup_y = (frame.area().height.saturating_sub(popup_height)) / 2;

    let area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the popup content
    let block = Block::default()
        .title(" Search ")
        .borders(Borders::ALL)
        .border_style(if error.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Cyan)
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build the input line with magnifying glass icon and ghost text
    let display_text = if input.is_empty() {
        "🔍 Enter search query (regex supported)"
    } else {
        input
    };

    let input_style = if input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    // Create input paragraph with icon
    let input_paragraph = if input.is_empty() {
        Paragraph::new(display_text).style(input_style)
    } else {
        Paragraph::new(format!("🔍 {}", input)).style(input_style)
    };

    frame.render_widget(input_paragraph, inner);
}
