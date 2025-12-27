pub mod layout;
pub mod widgets;

use crate::app::AppState;
use crate::error::Result;
use crate::types::{FocusTarget, UiMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> Result<()> {
    // Route input based on UI mode first
    match &app.ui_mode {
        UiMode::SearchPopup => {
            handle_search_popup_input(app, key)?;
        }
        UiMode::BookmarkPrompt => {
            handle_bookmark_prompt_input(app, key)?;
        }
        UiMode::BookPicker => {
            handle_book_picker_input(app, key)?;
        }
        UiMode::Help => {
            handle_help_input(app, key)?;
        }
        UiMode::MetadataPopup => {
            handle_metadata_popup_input(app, key)?;
        }
        UiMode::ErrorPopup(_) => {
            handle_error_popup_input(app, key)?;
        }
        UiMode::Normal => {
            // Route based on focus
            match app.focus {
                FocusTarget::TOC if app.toc_panel_visible => {
                    handle_toc_input(app, key)?;
                }
                FocusTarget::Bookmarks if app.bookmarks_panel_visible => {
                    handle_bookmarks_input(app, key)?;
                }
                _ => {
                    handle_content_input(app, key)?;
                }
            }
        }
    }
    Ok(())
}

fn handle_search_popup_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            if !app.input_buffer.is_empty() {
                // Perform search
                if let Some(book) = &mut app.book {
                    match crate::search::SearchEngine::search(book, &app.input_buffer) {
                        Ok(results) => {
                            app.search_query = app.input_buffer.clone();
                            app.search_results = results;
                            app.current_search_idx = 0;
                            
                            // Apply highlights
                            crate::search::SearchEngine::apply_highlights(book, &app.search_results);
                            
                            // Jump to first result if any
                            if !app.search_results.is_empty() {
                                app.next_search_result();
                            }
                            
                            app.ui_mode = UiMode::Normal;
                            app.input_buffer.clear();
                        }
                        Err(_) => {
                            // Keep popup open on error
                        }
                    }
                }
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            if app.input_buffer.len() < 500 {
                app.input_buffer.push(c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_bookmark_prompt_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            if !app.input_buffer.is_empty() {
                // Add bookmark
                let result = crate::bookmarks::BookmarkManager::add_bookmark(
                    &mut app.bookmarks,
                    app.current_chapter,
                    app.cursor_line,
                    app.input_buffer.clone(),
                );
                
                if result.is_ok() {
                    app.ui_mode = UiMode::Normal;
                    app.input_buffer.clear();
                }
                // If error, keep popup open
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            if app.input_buffer.len() < 100 {
                app.input_buffer.push(c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_book_picker_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            if app.book.is_none() {
                // No book loaded, exit app
                app.should_quit = true;
            } else {
                // Book loaded, close picker
                app.ui_mode = UiMode::Normal;
            }
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(idx) = app.book_picker_selected_idx {
                let next_idx = (idx + 1).min(app.recent_books.len().saturating_sub(1));
                app.book_picker_selected_idx = Some(next_idx);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(idx) = app.book_picker_selected_idx {
                app.book_picker_selected_idx = Some(idx.saturating_sub(1));
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.book_picker_selected_idx {
                if let Some(book_path) = app.recent_books.get(idx).cloned() {
                    // Load the selected book
                    match app.load_book_with_path(book_path.clone()) {
                        Ok(_) => {
                            // Render all chapters
                            if let Some(book) = &mut app.book {
                                for chapter in &mut book.chapters {
                                    crate::epub::render_chapter(chapter, app.config.max_width, app.viewport.width);
                                }
                            }
                            app.ui_mode = UiMode::Normal;
                            app.focus = FocusTarget::Content;
                        }
                        Err(e) => {
                            app.ui_mode = UiMode::ErrorPopup(format!("Failed to load book: {}", e));
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_help_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::F(1) => {
            app.ui_mode = UiMode::Normal;
            if let Some(prev_focus) = app.previous_focus.take() {
                app.focus = prev_focus;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_metadata_popup_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('I') => {
            app.ui_mode = UiMode::Normal;
            if let Some(prev_focus) = app.previous_focus.take() {
                app.focus = prev_focus;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_error_popup_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
    Ok(())
}

fn handle_bookmarks_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Bookmark navigation
        KeyCode::Char('j') | KeyCode::Down => {
            app.bookmark_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.bookmark_previous();
        }
        KeyCode::Enter => {
            app.jump_to_selected_bookmark();
        }
        KeyCode::Char('d') => {
            app.delete_selected_bookmark();
        }

        // Panel toggles
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_titlebar();
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_statusbar();
        }
        KeyCode::Char('t') => {
            app.toggle_toc();
        }
        KeyCode::Char('b') => {
            app.toggle_bookmarks();
        }
        KeyCode::Char('z') => {
            app.toggle_zen_mode();
        }

        // Focus management
        KeyCode::Tab => {
            app.cycle_focus();
        }
        KeyCode::Char('1') => {
            app.focus_toc();
        }
        KeyCode::Char('2') => {
            app.focus_content();
        }
        KeyCode::Char('3') => {
            app.focus_bookmarks();
        }

        _ => {}
    }
    Ok(())
}

fn handle_toc_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // TOC navigation
        KeyCode::Char('j') | KeyCode::Down => {
            app.toc_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.toc_previous();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.toc_open();
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.toc_close();
        }
        KeyCode::Enter => {
            app.toc_select();
        }

        // Panel toggles
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_titlebar();
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_statusbar();
        }
        KeyCode::Char('t') => {
            app.toggle_toc();
        }
        KeyCode::Char('b') => {
            app.toggle_bookmarks();
        }
        KeyCode::Char('z') => {
            app.toggle_zen_mode();
        }

        // Search
        KeyCode::Char('/') => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::SearchPopup;
            app.input_buffer.clear();
        }

        // Bookmarks
        KeyCode::Char('m') | KeyCode::Char('M') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::BookmarkPrompt;
            app.input_buffer.clear();
        }

        // Focus management
        KeyCode::Tab => {
            app.cycle_focus();
        }
        KeyCode::Char('1') => {
            app.focus_toc();
        }
        KeyCode::Char('2') => {
            app.focus_content();
        }
        KeyCode::Char('3') => {
            app.focus_bookmarks();
        }

        _ => {}
    }
    Ok(())
}

fn handle_content_input(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Half page scrolling with Ctrl+arrows (must come before regular arrow keys)
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_down();
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_up();
        }

        // Scrolling (j/k moves viewport, cursor follows)
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_down(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_up(1);
        }

        // Chapter navigation with Ctrl+PageUp/PageDown (must come before regular PageUp/PageDown)
        KeyCode::PageUp if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.previous_chapter();
        }
        KeyCode::PageDown if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.next_chapter();
        }

        // Page scrolling - Space and PageDown
        KeyCode::Char(' ') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.page_up();
            } else {
                app.page_down();
            }
        }
        KeyCode::PageDown => {
            app.page_down();
        }
        // Ctrl-f for page down
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_down();
        }
        // Ctrl-b and PageUp for page up
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.page_up();
        }
        KeyCode::PageUp => {
            app.page_up();
        }
        // Half page scrolling with Ctrl+d/u
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_down();
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_up();
        }

        // Panel and UI toggles (Ctrl+t/s must come before 't')
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_titlebar();
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_statusbar();
        }
        KeyCode::Char('t') => {
            app.toggle_toc();
        }
        KeyCode::Char('b') => {
            app.toggle_bookmarks();
        }
        KeyCode::Char('z') => {
            app.toggle_zen_mode();
        }

        // Search
        KeyCode::Char('/') => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::SearchPopup;
            app.input_buffer.clear();
        }
        KeyCode::Char('n') => {
            app.next_search_result();
        }
        KeyCode::Char('N') => {
            app.previous_search_result();
        }

        // Bookmarks
        KeyCode::Char('m') | KeyCode::Char('M') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::BookmarkPrompt;
            app.input_buffer.clear();
        }

        // Help
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::Help;
        }

        // Metadata popup
        KeyCode::Char('I') => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::MetadataPopup;
        }

        // Book picker
        KeyCode::Char('o') | KeyCode::Char('O') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.previous_focus = Some(app.focus.clone());
            app.ui_mode = UiMode::BookPicker;
            
            // Set selection to current book if available
            if let Some(current_path) = &app.current_book_path {
                app.book_picker_selected_idx = app.recent_books
                    .iter()
                    .position(|p| p == current_path)
                    .or(Some(0));
            } else {
                app.book_picker_selected_idx = Some(0);
            }
        }

        // Cursor movement
        KeyCode::Char('H') => {
            app.move_cursor_to_top();
        }
        KeyCode::Char('M') => {
            app.move_cursor_to_middle();
        }
        KeyCode::Char('L') => {
            app.move_cursor_to_bottom();
        }
        KeyCode::Char('g') | KeyCode::Home => {
            app.move_cursor_to_chapter_start();
        }
        KeyCode::Char('G') | KeyCode::End => {
            app.move_cursor_to_chapter_end();
        }

        // Chapter navigation
        KeyCode::Char('{') => {
            app.previous_chapter();
        }
        KeyCode::Char('}') => {
            app.next_chapter();
        }

        // Section navigation
        KeyCode::Char('[') => {
            app.previous_section();
        }
        KeyCode::Char(']') => {
            app.next_section();
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            app.previous_section();
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            app.next_section();
        }

        // Focus management
        KeyCode::Tab => {
            app.cycle_focus();
        }
        KeyCode::Char('1') => {
            app.focus_toc();
        }
        KeyCode::Char('2') => {
            app.focus_content();
        }
        KeyCode::Char('3') => {
            app.focus_bookmarks();
        }

        _ => {}
    }
    Ok(())
}
