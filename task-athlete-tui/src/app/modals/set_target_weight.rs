// src/app/modals/set_target_weight.rs

use crate::app::state::{ActiveModal, App, SetTargetWeightField};
use crate::app::utils::parse_modal_weight;
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

// --- Submission Logic ---

fn submit_set_target_weight(app: &mut App, weight_input: &str) -> Result<(), AppInputError> {
    let weight = parse_modal_weight(weight_input)?;
    match app.service.set_target_bodyweight(Some(weight)) {
        Ok(()) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error setting target: {e}" // ConfigError usually doesn't need DbError type
        ))),
    }
}

fn submit_clear_target_weight(app: &mut App) -> Result<(), AppInputError> {
    match app.service.set_target_bodyweight(None) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error clearing target: {e}"
        ))),
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_set_target_weight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut submit_action: Option<fn(&mut App, &str) -> Result<(), AppInputError>> = None; // For Set
    let mut clear_action: Option<fn(&mut App) -> Result<(), AppInputError>> = None; // For Clear

    if let ActiveModal::SetTargetWeight {
        ref mut weight_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        *error_message = None; // Clear error on any input

        match focused_field {
            SetTargetWeightField::Weight => match key.code {
                KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                KeyCode::Backspace => {
                    weight_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = SetTargetWeightField::Set
                }
                KeyCode::Up => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Set => match key.code {
                KeyCode::Enter => {
                    // Prepare to submit *after* this block
                    weight_to_submit = weight_input.clone();
                    submit_action = Some(submit_set_target_weight);
                }
                KeyCode::Right | KeyCode::Tab => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Up => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Down => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Clear => match key.code {
                KeyCode::Enter => {
                    // Prepare to clear *after* this block
                    clear_action = Some(submit_clear_target_weight);
                }
                KeyCode::Left => *focused_field = SetTargetWeightField::Set,
                KeyCode::Right | KeyCode::Tab => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Up => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Down => *focused_field = SetTargetWeightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            SetTargetWeightField::Cancel => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                KeyCode::Left => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Tab => *focused_field = SetTargetWeightField::Weight, // Wrap around
                KeyCode::Up => *focused_field = SetTargetWeightField::Clear,
                KeyCode::Down => *focused_field = SetTargetWeightField::Weight, // Wrap around
                _ => {}
            },
        }
    } // Mutable borrow of app.active_modal ends here

    // --- Submission/Clear Logic ---
    let mut submit_result: Result<(), AppInputError> = Ok(()); // Default to Ok

    if let Some(action) = submit_action {
        submit_result = action(app, &weight_to_submit);
    } else if let Some(action) = clear_action {
        submit_result = action(app);
    }

    // Only process result if an action was attempted
    if submit_action.is_some() || clear_action.is_some() {
        if submit_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Refresh handled by main loop
        } else {
            // Re-borrow ONLY if necessary to set error
            if let ActiveModal::SetTargetWeight {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submit_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}
