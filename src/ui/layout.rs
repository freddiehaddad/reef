use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Titlebar
            Constraint::Min(0),     // Content
            Constraint::Length(1), // Statusbar
        ])
        .split(f.area());

    render_titlebar(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
    render_statusbar(f, app, chunks[2]);
}

fn render_titlebar(f: &mut Frame, app: &AppState, area: Rect) {
    let title_text = if let Some(book) = &app.book {
        let chapter_title = app.get_current_chapter()
            .map(|ch| ch.title.as_str())
            .unwrap_or("Unknown Chapter");
        
        format!("{} - {}", book.metadata.title, chapter_title)
    } else {
        "EPUB Reader".to_string()
    };

    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, app: &AppState, area: Rect) {
    if let Some(chapter) = app.get_current_chapter() {
        let visible_start = app.viewport.scroll_offset;
        let visible_end = (visible_start + area.height as usize).min(chapter.content_lines.len());
        
        let mut lines = Vec::new();
        
        for (idx, line) in chapter.content_lines[visible_start..visible_end].iter().enumerate() {
            let global_line_idx = visible_start + idx;
            
            // Highlight cursor line with subtle background
            let style = if global_line_idx == app.cursor_line {
                Style::default().bg(Color::Rgb(40, 40, 50))
            } else {
                Style::default()
            };
            
            lines.push(Line::from(Span::styled(line.text.clone(), style)));
        }
        
        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false });
        
        f.render_widget(paragraph, area);
    } else {
        let text = Paragraph::new("No book loaded")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(text, area);
    }
}

fn render_statusbar(f: &mut Frame, app: &AppState, area: Rect) {
    let status_text = if app.book.is_some() {
        let current_ch = app.current_chapter + 1;
        let total_ch = app.total_chapters();
        let current_line = app.cursor_line + 1;
        let total_lines = app.current_chapter_lines();
        
        let percentage = if total_lines > 0 {
            (app.cursor_line * 100) / total_lines
        } else {
            0
        };
        
        format!(
            "Ch {}/{} | Line {}/{} ({}%)",
            current_ch, total_ch, current_line, total_lines, percentage
        )
    } else {
        "No book loaded".to_string()
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    
    f.render_widget(status, area);
}
