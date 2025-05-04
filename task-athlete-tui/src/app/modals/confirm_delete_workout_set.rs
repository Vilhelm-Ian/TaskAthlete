// src/app/modals/confirm_delete_workout_set.rs

use crate::app::state::{ActiveModal, App};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// --- Submission Logic ---

fn submit_delete_workout_set(app: &mut App, workout_id: u64) -> Result<(), AppInputError> {
    match app.service.delete_workouts(&vec![workout_id as i64]) {
        Ok(_) => {
            // Adjust selection after deletion if necessary
            if let Some(selected_index) = app.log_set_table_state.selected() {
                if selected_index >= app.log_sets_for_selected_exercise.len().saturating_sub(1) {
                    // Adjust if last item deleted
                    let new_index = app.log_sets_for_selected_exercise.len().saturating_sub(2); // Select new last item
                    app.log_set_table_state.select(
                        if new_index > 0 || app.log_sets_for_selected_exercise.len() == 1 {
                            Some(new_index)
                        } else {
                            None
                        },
                    );
                }
            }
            Ok(())
        }
        Err(e) => Err(AppInputError::DbError(format!(
            "Error deleting workout: {}",
            e
        ))),
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_confirm_delete_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut should_delete = false;
    let mut workout_id_to_delete: u64 = 0; // Placeholder

    // ActiveModal::ConfirmDeleteWorkout does not have focused_field
    if let ActiveModal::ConfirmDeleteWorkout { workout_id, .. } = app.active_modal {
        workout_id_to_delete = workout_id; // Capture the ID
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
        // If modal state isn't ConfirmDeleteWorkout, something is wrong.
        // Close it to be safe.
        app.active_modal = ActiveModal::None;
        return Ok(());
    }

    if should_delete {
        let delete_result = submit_delete_workout_set(app, workout_id_to_delete);
        if delete_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
        } else {
            // If delete fails, show error in status bar (modal is already closed or will be replaced)
            // Or, we could potentially transition to an Error modal, but status bar is simpler.
            app.set_error(delete_result.unwrap_err().to_string());
            app.active_modal = ActiveModal::None; // Close the confirmation modal even on error
        }
    }

    Ok(())
}
