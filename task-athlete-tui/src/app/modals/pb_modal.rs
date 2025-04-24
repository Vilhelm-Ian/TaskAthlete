// src/app/modals/pb_modal.rs

use crate::app::state::{ActiveModal, App};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_pb_modal_input(app: &mut App, key: KeyEvent) -> Result<(), AppInputError> {
    // In this simple modal, Enter or Esc just closes it.
    // We still use focused_field for consistency if we added more buttons later.
    // PbModal currently has no focused_field, so we handle key directly.
    if let ActiveModal::PersonalBest { .. } = app.active_modal {
        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                app.active_modal = ActiveModal::None;
            }
            // If there were multiple buttons, handle Left/Right navigation here,
            // using a focused_field on the PbModal state struct.
            _ => {} // Ignore other keys
        }
    } else {
        // If modal state isn't PersonalBest, something is wrong.
        // Close it to be safe.
        app.active_modal = ActiveModal::None;
    }
    Ok(())
}
