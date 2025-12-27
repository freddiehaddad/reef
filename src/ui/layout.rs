use crate::app::AppState;
use crate::types::{FocusTarget, LineStyle, UiMode};
use crate::ui::widgets;
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
    
    // Render content area (may include TOC and Bookmarks panels)
    let content_area = main_chunks[chunk_idx];
    chunk_idx += 1;
    
    let mut toc_bookmarks_constraints = Vec::new();
    if app.toc_panel_visible {
        toc_bookmarks_constraints.push(Constraint::Length(app.config.toc_panel_width));
    }
    toc_bookmarks_constraints.push(Constraint::Min(0)); // Main content
    if app.bookmarks_panel_visible {
        toc_bookmarks_constraints.push(Constraint::Length(app.config.bookmarks_panel_width));
    }
    
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(toc_bookmarks_constraints)
        .split(content_area);
    
    let mut chunk_index = 0;
    if app.toc_panel_visible {
        render_toc(f, app, content_chunks[chunk_index]);
        chunk_index += 1;
    }
    render_content(f, app, content_chunks[chunk_index]);
    chunk_index += 1;
    if app.bookmarks_panel_visible {
        render_bookmarks(f, app, content_chunks[chunk_index]);
    }
    
    // Render statusbar if visible
    if app.statusbar_visible {
        render_statusbar(f, app, main_chunks[chunk_idx]);
    }
    
    // Render popups on top
    match &app.ui_mode {
        UiMode::SearchPopup => {
            // Check for regex validation error
            let error = if !app.input_buffer.is_empty() {
                regex::Regex::new(&app.input_buffer).err().map(|e| format!("Invalid regex: {}", e))
            } else {
                None
            };
            widgets::popups::search::render_search_popup(f, &app.input_buffer, error.as_deref());
        }
        UiMode::BookmarkPrompt => {
            // Generate suggestion
            let suggestion = if let Some(chapter) = app.get_current_chapter() {
                if let Some(line) = chapter.content_lines.get(app.cursor_line) {
                    crate::bookmarks::BookmarkManager::generate_label_suggestion(
                        &line.text,
                        &chapter.title,
                    )
                } else {
                    None
                }
            } else {
                None
            };
            
            // Check for empty label error
            let error = if app.input_buffer.trim().is_empty() && !app.input_buffer.is_empty() {
                Some("Label cannot be empty")
            } else {
                None
            };
            
            widgets::popups::bookmark_prompt::render_bookmark_prompt(
                f,
                &app.input_buffer,
                suggestion.as_deref(),
                error,
            );
        }
        UiMode::BookPicker => {
            widgets::popups::book_picker::render_book_picker(
                f,
                &app.recent_books,
                app.book_picker_selected_idx,
            );
        }
        UiMode::Help => {
            // TODO: Implement help popup
        }
        UiMode::MetadataPopup => {
            if let Some(book) = &app.book {
                widgets::popups::metadata::render_metadata_popup(f, &book.metadata);
            }
        }
        UiMode::ErrorPopup(message) => {
            // TODO: Implement error popup
            let _ = message; // Silence unused warning
        }
        UiMode::Normal => {}
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
            
            // Apply search highlighting if there are matches in this line
            if !line.search_matches.is_empty() {
                // Build line with search highlights
                let mut spans = Vec::new();
                let mut last_pos = 0;
                
                for (start, end) in &line.search_matches {
                    // Add text before match
                    if *start > last_pos {
                        let base_style = get_line_style(&line.style, global_line_idx, app.cursor_line);
                        spans.push(Span::styled(
                            line.text[last_pos..*start].to_string(),
                            base_style,
                        ));
                    }
                    
                    // Determine if this is the current search result
                    let is_current_match = if !app.search_results.is_empty() {
                        let current_result = &app.search_results[app.current_search_idx];
                        current_result.chapter_idx == app.current_chapter
                            && current_result.line == global_line_idx
                            && current_result.column == *start
                    } else {
                        false
                    };
                    
                    // Add highlighted match
                    let highlight_color = if is_current_match {
                        Color::Rgb(255, 200, 100) // Current match: bright yellow/orange
                    } else {
                        Color::Rgb(200, 150, 50) // Other matches: darker yellow
                    };
                    
                    let mut match_style = get_line_style(&line.style, global_line_idx, app.cursor_line);
                    match_style = match_style.bg(highlight_color).fg(Color::Black);
                    
                    spans.push(Span::styled(
                        line.text[*start..*end].to_string(),
                        match_style,
                    ));
                    
                    last_pos = *end;
                }
                
                // Add remaining text after last match
                if last_pos < line.text.len() {
                    let base_style = get_line_style(&line.style, global_line_idx, app.cursor_line);
                    spans.push(Span::styled(
                        line.text[last_pos..].to_string(),
                        base_style,
                    ));
                }
                
                lines.push(Line::from(spans));
            } else {
                // No search matches, render normally
                let base_style = get_line_style(&line.style, global_line_idx, app.cursor_line);
                lines.push(Line::from(Span::styled(line.text.clone(), base_style)));
            }
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

fn get_line_style(line_style: &LineStyle, line_idx: usize, cursor_line: usize) -> Style {
    let mut base_style = match line_style {
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
    if line_idx == cursor_line {
        base_style = base_style.bg(Color::Rgb(40, 40, 50));
    }
    
    base_style
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
    
    // Append search info if active
    let full_status = if !app.search_results.is_empty() {
        let query_display = if app.search_query.len() > 20 {
            format!("{}...", &app.search_query[..17])
        } else {
            app.search_query.clone()
        };
        format!(
            "{} | [Search: '{}' {}/{}]",
            status_text,
            query_display,
            app.current_search_idx + 1,
            app.search_results.len()
        )
    } else {
        status_text
    };

    let status = Paragraph::new(full_status)
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

fn render_bookmarks(f: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.focus == FocusTarget::Bookmarks;
    
    let panel = widgets::bookmarks::BookmarksPanel::new(
        &app.bookmarks,
        app.selected_bookmark_idx,
        is_focused,
    );
    
    panel.render(f, area);
}
