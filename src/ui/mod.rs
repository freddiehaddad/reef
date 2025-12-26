pub mod layout;

use crate::app::AppState;
use crate::error::Result;
use crate::types::FocusTarget;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> Result<()> {
    // Route input based on focus
    match app.focus {
        FocusTarget::TOC if app.toc_panel_visible => {
            handle_toc_input(app, key)?;
        }
        _ => {
            handle_content_input(app, key)?;
        }
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

        _ => {}
    }
    Ok(())
}
