use crate::app::AppState;
use crate::constants::{MAX_BOOKMARK_INPUT_LENGTH, MAX_SEARCH_INPUT_LENGTH};
use crate::error::Result;
use crate::types::{FocusTarget, UiMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct InputHandler;

impl InputHandler {
    /// Handle common panel toggles and UI controls
    /// Returns true if the key was handled, false otherwise
    fn handle_common_controls(app: &mut AppState, key: KeyEvent) -> bool {
        match key.code {
            // Quit
            KeyCode::Char('q') => {
                app.should_quit = true;
                true
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true;
                true
            }
            // Panel toggles
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_titlebar();
                true
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_statusbar();
                true
            }
            KeyCode::Char('t') => {
                app.toggle_toc();
                true
            }
            KeyCode::Char('b') => {
                app.toggle_bookmarks();
                true
            }
            KeyCode::Char('z') => {
                app.toggle_zen_mode();
                true
            }
            // Focus management
            KeyCode::Tab => {
                app.cycle_focus();
                true
            }
            KeyCode::Char('1') => {
                app.focus_toc();
                true
            }
            KeyCode::Char('2') => {
                app.focus_content();
                true
            }
            KeyCode::Char('3') => {
                app.focus_bookmarks();
                true
            }
            _ => false,
        }
    }

    pub fn handle_key(&mut self, app: &mut AppState, key: KeyEvent) -> Result<()> {
        // Route input based on UI mode first
        match &app.ui_mode {
            UiMode::SearchPopup => Self::handle_search_popup(app, key),
            UiMode::BookmarkPrompt => Self::handle_bookmark_prompt(app, key),
            UiMode::BookPicker => Self::handle_book_picker(app, key),
            UiMode::Help => Self::handle_help(app, key),
            UiMode::MetadataPopup => Self::handle_metadata_popup(app, key),
            UiMode::ErrorPopup(_) => Self::handle_error_popup(app, key),
            UiMode::Normal => {
                // Route based on focus
                match app.focus {
                    FocusTarget::Toc if app.toc_panel_visible => Self::handle_toc(app, key),
                    FocusTarget::Bookmarks if app.bookmarks_panel_visible => {
                        Self::handle_bookmarks(app, key)
                    }
                    _ => Self::handle_content(app, key),
                }
            }
        }
    }

    fn handle_search_popup(app: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                log::debug!("Search cancelled by user");
                app.ui_mode = UiMode::Normal;
                app.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !app.input_buffer.is_empty() {
                    log::info!("Executing search: query='{}'", app.input_buffer);
                    // Perform search
                    if let Some(book) = &mut app.book {
                        match crate::search::SearchEngine::search(book, &app.input_buffer) {
                            Ok(results) => {
                                log::info!("Search completed: {} results found", results.len());
                                app.search_query = app.input_buffer.clone();
                                app.search_results = results;
                                app.current_search_idx = 0;

                                // Apply highlights
                                crate::search::SearchEngine::apply_highlights(
                                    book,
                                    &app.search_results,
                                );

                                // Jump to first result if any
                                if !app.search_results.is_empty() {
                                    log::debug!("Jumping to first search result");
                                    app.next_search_result();
                                }

                                app.ui_mode = UiMode::Normal;
                                app.input_buffer.clear();
                            }
                            Err(e) => {
                                log::warn!("Search failed: {}", e);
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
                if app.input_buffer.len() < MAX_SEARCH_INPUT_LENGTH {
                    app.input_buffer.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_bookmark_prompt(app: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                log::debug!("Bookmark creation cancelled by user");
                app.ui_mode = UiMode::Normal;
                app.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !app.input_buffer.is_empty() {
                    log::debug!("Creating bookmark: label='{}'", app.input_buffer);
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
                    } else if let Err(e) = result {
                        log::warn!("Bookmark creation failed: {}", e);
                    }
                    // If error, keep popup open
                }
            }
            KeyCode::Backspace => {
                app.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                if app.input_buffer.len() < MAX_BOOKMARK_INPUT_LENGTH {
                    app.input_buffer.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_book_picker(app: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                log::debug!("Book picker closed");
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
                if let Some(idx) = app.book_picker_selected_idx
                    && let Some(book_path) = app.recent_books.get(idx).cloned()
                {
                    log::info!("Loading book from picker: {}", book_path);
                    // Load the selected book
                    match app.load_book_with_path(book_path.clone()) {
                        Ok(_) => {
                            log::debug!("Book loaded, rendering all chapters");
                            // Render all chapters
                            let effective_width = app.effective_max_width();
                            let viewport_width = app.viewport.width;
                            if let Some(book) = &mut app.book {
                                for chapter in &mut book.chapters {
                                    crate::epub::render_chapter(
                                        chapter,
                                        effective_width,
                                        viewport_width,
                                    );
                                }
                            }
                            app.ui_mode = UiMode::Normal;
                            app.focus = FocusTarget::Content;
                        }
                        Err(e) => {
                            log::error!("Failed to load book from picker: {}", e);
                            app.ui_mode = UiMode::ErrorPopup(format!("Failed to load book: {}", e));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_help(app: &mut AppState, key: KeyEvent) -> Result<()> {
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

    fn handle_metadata_popup(app: &mut AppState, key: KeyEvent) -> Result<()> {
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

    fn handle_error_popup(app: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.ui_mode = UiMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_bookmarks(app: &mut AppState, key: KeyEvent) -> Result<()> {
        // Try common controls first
        if Self::handle_common_controls(app, key) {
            return Ok(());
        }

        // Bookmark-specific controls
        match key.code {
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
            _ => {}
        }
        Ok(())
    }

    fn handle_toc(app: &mut AppState, key: KeyEvent) -> Result<()> {
        // Try common controls first
        if Self::handle_common_controls(app, key) {
            return Ok(());
        }

        // TOC-specific controls
        match key.code {
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
            // Search
            KeyCode::Char('/') => {
                app.previous_focus = Some(app.focus.clone());
                app.ui_mode = UiMode::SearchPopup;
                app.input_buffer.clear();
            }
            // Bookmarks
            KeyCode::Char('m') | KeyCode::Char('M')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.previous_focus = Some(app.focus.clone());
                app.ui_mode = UiMode::BookmarkPrompt;
                app.input_buffer.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_content(app: &mut AppState, key: KeyEvent) -> Result<()> {
        // Try common controls first
        if Self::handle_common_controls(app, key) {
            return Ok(());
        }

        match key.code {
            // Clear search highlights
            KeyCode::Esc => {
                if !app.search_results.is_empty() {
                    // Clear highlights from book
                    if let Some(book) = &mut app.book {
                        crate::search::SearchEngine::clear_highlights(book);
                    }

                    // Clear search state
                    app.search_results.clear();
                    app.search_query.clear();
                    app.current_search_idx = 0;
                }
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

            // Cycle max width
            KeyCode::Char('w') => {
                app.cycle_max_width();
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
            KeyCode::Char('m') | KeyCode::Char('M')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
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
            KeyCode::Char('o') | KeyCode::Char('O')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.previous_focus = Some(app.focus.clone());
                app.ui_mode = UiMode::BookPicker;

                // Set selection to current book if available
                if let Some(current_path) = &app.current_book_path {
                    app.book_picker_selected_idx = app
                        .recent_books
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

            _ => {}
        }
        Ok(())
    }
}
