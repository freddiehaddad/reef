use crate::app::AppState;
use crate::types::{FocusTarget, LineStyle};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tui_tree_widget::Tree;

pub fn render(f: &mut Frame, app: &mut AppState) {
    // Calculate constraints based on visibility
    let mut constraints = Vec::new();
    
    if app.titlebar_visible {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(0)); // Content area
    if app.statusbar_visible {
        constraints.push(Constraint::Length(1));
    }
    
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut chunk_idx = 0;
    
    // Render titlebar if visible
    if app.titlebar_visible {
        render_titlebar(f, app, main_chunks[chunk_idx]);
        chunk_idx += 1;
    }
    
    // Render content area (may include TOC panel)
    let content_area = main_chunks[chunk_idx];
    chunk_idx += 1;
    
    if app.toc_panel_visible {
        // Split content area for TOC and main content
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(app.config.toc_panel_width),
                Constraint::Min(0),
            ])
            .split(content_area);
        
        render_toc(f, app, content_chunks[0]);
        render_content(f, app, content_chunks[1]);
    } else {
        render_content(f, app, content_area);
    }
    
    // Render statusbar if visible
    if app.statusbar_visible {
        render_statusbar(f, app, main_chunks[chunk_idx]);
    }
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
            
            // Determine style based on line type
            let mut base_style = match &line.style {
                LineStyle::Heading1 => Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                LineStyle::Heading2 => Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                LineStyle::Heading3 => Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                LineStyle::CodeBlock { .. } => Style::default()
                    .fg(Color::Green),
                LineStyle::InlineCode => Style::default()
                    .fg(Color::Yellow),
                LineStyle::Quote => Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
                LineStyle::Link => Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
                LineStyle::Normal => Style::default(),
            };
            
            // Add cursor background highlight
            if global_line_idx == app.cursor_line {
                base_style = base_style.bg(Color::Rgb(40, 40, 50));
            }
            
            lines.push(Line::from(Span::styled(line.text.clone(), base_style)));
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
        
        // Determine current section
        let section_info = if let Some(chapter) = app.get_current_chapter() {
            if !chapter.sections.is_empty() {
                // Find which section contains the cursor
                let mut current_section_idx = None;
                for (idx, section) in chapter.sections.iter().enumerate() {
                    let next_start = chapter.sections.get(idx + 1)
                        .map(|s| s.start_line)
                        .unwrap_or(usize::MAX);
                    if section.start_line <= app.cursor_line && app.cursor_line < next_start {
                        current_section_idx = Some(idx + 1);
                        break;
                    }
                }
                
                if let Some(sec_idx) = current_section_idx {
                    format!(" | Sec {}/{}", sec_idx, chapter.sections.len())
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        format!(
            "Ch {}/{}{}  | Line {}/{} ({}%)",
            current_ch, total_ch, section_info, current_line, total_lines, percentage
        )
    } else {
        "No book loaded".to_string()
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    
    f.render_widget(status, area);
}

fn render_toc(f: &mut Frame, app: &mut AppState, area: Rect) {
    let is_focused = app.focus == FocusTarget::TOC;
    
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title("TOC");
    
    let tree = Tree::new(&app.toc_state.items)
        .expect("Failed to create tree")
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    
    f.render_stateful_widget(tree, area, &mut app.toc_state.tree_state);
}
