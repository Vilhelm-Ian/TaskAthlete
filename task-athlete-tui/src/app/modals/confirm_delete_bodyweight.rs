// src/app/modals/confirm_delete_bodyweight.rs

use crate::app::state::{ActiveModal, App};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// --- Submission Logic ---

fn sumbit_delete_body_weight(app: &mut App, bodyweight_id: u64) -> Result<(), AppInputError> {
    match app.service.delete_bodyweight(bodyweight_id as i64) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error deleting bodyweight: {}",
            e
        ))),
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_confirm_delete_body_weigth_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut should_delete = false;
    let mut bodyweight_id_to_delete: u64 = 0; // Placeholder

    // ActiveModal::ConfirmDeleteBodyWeight does not have focused_field
    if let ActiveModal::ConfirmDeleteBodyWeight { body_weight_id, .. } = app.active_modal {
        bodyweight_id_to_delete = body_weight_id; // Capture the ID
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                should_delete = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Backspace => {
                app.active_modal = ActiveModal::None; // Close modal, do nothing
                return Ok(());
            }
            _ => {} // Ignore other keys
        }
    } else {
        // If modal state isn't ConfirmDeleteBodyWeight, something is wrong.
        // Close it to be safe.
        app.active_modal = ActiveModal::None;
        return Ok(());
    }

    if should_delete {
        let delete_result = sumbit_delete_body_weight(app, bodyweight_id_to_delete);
        if delete_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
        } else {
            // If delete fails, show error in status bar (modal is already closed or will be replaced)
            app.set_error(delete_result.unwrap_err().to_string());
            app.active_modal = ActiveModal::None; // Close the confirmation modal even on error
        }
    }

    Ok(())
}
