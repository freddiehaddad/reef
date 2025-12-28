use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn render_book_picker(f: &mut Frame, books: &[String], selected_idx: Option<usize>) {
    // Create a centered popup (60% width, 60% height)
    let area = centered_rect(60, 60, f.area());
    
    // Clear the area behind the popup
    f.render_widget(Clear, area);
    
    // Create the block
    let block = Block::default()
        .title("Recent Books")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    
    if books.is_empty() {
        let message = Paragraph::new("No recent books.\n\nOpen a book with: epub-reader <file.epub>")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(message, inner_area);
    } else {
        // Create list items
        let items: Vec<ListItem> = books
            .iter()
            .enumerate()
            .map(|(idx, path)| {
                // Extract filename from path
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                
                let is_selected = selected_idx == Some(idx);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                ListItem::new(filename).style(style)
            })
            .collect();
        
        let list = List::new(items);
        f.render_widget(list, inner_area);
    }
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
