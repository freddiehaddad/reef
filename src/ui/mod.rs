pub mod layout;
pub mod widgets;

use crate::app::AppState;
use crate::error::Result;
use crate::types::{FocusTarget, UiMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> Result<()> {
    // Route input based on UI mode first
    match app.ui_mode {
        UiMode::SearchPopup => {
            handle_search_popup_input(app, key)?;
        }
        UiMode::BookmarkPrompt => {
            handle_bookmark_prompt_input(app, key)?;
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
