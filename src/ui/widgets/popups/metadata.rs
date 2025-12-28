use crate::types::BookMetadata;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_metadata_popup(f: &mut Frame, metadata: &BookMetadata) {
    // Create a centered popup (50% width, 50% height)
    let area = centered_rect(50, 50, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    // Create the block
    let block = Block::default()
        .title("Book Information")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Build info lines
    let mut lines = Vec::new();

    lines.push(Line::from(format!("Title: {}", metadata.title)));

    if let Some(author) = &metadata.author {
        lines.push(Line::from(format!("Author: {}", author)));
    }

    if let Some(publisher) = &metadata.publisher {
        lines.push(Line::from(format!("Publisher: {}", publisher)));
    }

    if let Some(date) = &metadata.publication_date {
        lines.push(Line::from(format!("Publication Date: {}", date)));
    }

    if let Some(language) = &metadata.language {
        lines.push(Line::from(format!("Language: {}", language)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Press Esc or Shift-I to close").style(Style::default().fg(Color::Gray)));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner_area);
}

// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
