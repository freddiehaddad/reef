pub mod layout;

use crate::app::AppState;
use crate::error::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Scrolling (j/k moves viewport, cursor follows)
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_down(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_up(1);
        }

        // Page scrolling - Space and PageDown
        KeyCode::Char(' ') | KeyCode::PageDown => {
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
        // Half page scrolling
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_down();
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.half_page_up();
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

        _ => {}
    }
    Ok(())
}
