use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render_error_popup(f: &mut Frame, message: &str, _area: Rect) {
    // Calculate popup size (40% width, auto height based on message)
    let popup_width = (f.area().width as f32 * 0.4) as u16;
    let popup_height = 7; // Enough for title, message, and OK button
    
    let popup_x = (f.area().width.saturating_sub(popup_width)) / 2;
    let popup_y = (f.area().height.saturating_sub(popup_height)) / 2;
    
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };
    
    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);
    
    let block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
    
    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    
    // Split inner area into message and button sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Message
            Constraint::Length(1),   // Spacer
            Constraint::Length(1),   // Button
        ])
        .split(inner_area);
    
    // Render message
    let error_text = Paragraph::new(message)
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    f.render_widget(error_text, chunks[0]);
    
    // Render OK button
    let button_text = Line::from(vec![
        Span::styled("[OK]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]);
    let button = Paragraph::new(button_text)
        .alignment(Alignment::Center);
    f.render_widget(button, chunks[2]);
}
