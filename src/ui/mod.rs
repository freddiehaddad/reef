//! User interface components and event handling
//!
//! This module contains all UI-related code including layout rendering,
//! widgets, and keyboard input handling.

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
