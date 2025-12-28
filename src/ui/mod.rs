pub mod input_handler;
pub mod layout;
pub mod widgets;

use crate::app::AppState;
use crate::error::Result;
use crossterm::event::KeyEvent;
use input_handler::InputHandler;

pub fn handle_key_event(app: &mut AppState, key: KeyEvent) -> Result<()> {
    InputHandler.handle_key(app, key)
}
