use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_help_popup(f: &mut Frame, _area: Rect) {
    // Calculate popup size (70% width, 80% height)
    let popup_width = (f.area().width as f32 * 0.7) as u16;
    let popup_height = (f.area().height as f32 * 0.8) as u16;
    
    let popup_x = (f.area().width.saturating_sub(popup_width)) / 2;
    let popup_y = (f.area().height.saturating_sub(popup_height)) / 2;
    
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };
    
    // Create help text
    let help_text = vec![
        Line::from(vec![
            Span::styled("NAVIGATION", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("  j / ↓              Scroll down one line"),
        Line::from("  k / ↑              Scroll up one line"),
        Line::from("  Ctrl-d / Ctrl-↓    Scroll down half page"),
        Line::from("  Ctrl-u / Ctrl-↑    Scroll up half page"),
        Line::from("  Space / PgDn       Scroll down full page"),
        Line::from("  Shift-Space / PgUp Scroll up full page"),
        Line::from("  H / M / L          Move cursor to top/middle/bottom"),
        Line::from("  g / Home           Move cursor to top of chapter"),
        Line::from("  G / End            Move cursor to bottom of chapter"),
        Line::from("  { / }              Previous/next chapter"),
        Line::from("  [ / ]              Previous/next section"),
        Line::from(""),
        Line::from(vec![
            Span::styled("PANELS & VIEWS", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("  t                  Toggle TOC panel"),
        Line::from("  b                  Toggle bookmarks panel"),
        Line::from("  Ctrl-s             Toggle statusbar"),
        Line::from("  Ctrl-t             Toggle titlebar"),
        Line::from("  z                  Zen mode (hide all UI)"),
        Line::from("  Shift-I            Show book metadata"),
        Line::from("  o / Ctrl-o         Open book picker"),
        Line::from(""),
        Line::from(vec![
            Span::styled("SEARCH & BOOKMARKS", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("  /                  Open search"),
        Line::from("  n / N              Next/previous search result"),
        Line::from("  m                  Add bookmark at cursor"),
        Line::from("  d                  Delete bookmark (in bookmarks panel)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("FOCUS MANAGEMENT", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("  Tab                Cycle focus between panels"),
        Line::from("  1 / 2 / 3          Focus TOC/Content/Bookmarks"),
        Line::from(""),
        Line::from(vec![
            Span::styled("APPLICATION", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from("  ? / F1             Toggle this help"),
        Line::from("  q / Ctrl-q         Quit"),
        Line::from("  Esc                Close popup/panel"),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press Esc or ? to close", Style::default().fg(Color::Gray)),
        ]),
    ];
    
    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    
    f.render_widget(paragraph, popup_area);
}
