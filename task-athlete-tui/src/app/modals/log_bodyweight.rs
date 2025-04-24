use crate::app::state::{ActiveModal, App, LogBodyweightField};
use crate::app::utils::{parse_modal_date, parse_modal_weight};
use crate::app::AppInputError;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use task_athlete_lib::DbError;

// --- Submission Logic ---

fn submit_log_bodyweight(
    app: &mut App, // Pass App mutably
    weight_input: &str,
    date_input: &str,
) -> Result<(), AppInputError> {
    let weight = parse_modal_weight(weight_input)?;
    let date = parse_modal_date(date_input)?;

    let timestamp = date
        .and_hms_opt(12, 0, 0)
        .and_then(|ndt| Utc.from_local_datetime(&ndt).single())
        .ok_or_else(|| AppInputError::InvalidDate("Internal date conversion error".into()))?;

    match app.service.add_bodyweight_entry(timestamp, weight) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(db_err) = e.downcast_ref::<DbError>() {
                if let DbError::BodyweightEntryExists(_) = db_err {
                    return Err(AppInputError::InvalidDate(
                        "Entry already exists for this date".to_string(),
                    ));
                }
                // Return specific DB error message if possible
                return Err(AppInputError::DbError(db_err.to_string()));
            }
            // Generic error for other DB issues
            Err(AppInputError::DbError(format!("DB Error: {}", e)))
        }
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_log_bodyweight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut date_to_submit = String::new();
    let mut should_submit = false;

    if let ActiveModal::LogBodyweight {
        ref mut weight_input,
        ref mut date_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;

        match focused_field {
            LogBodyweightField::Weight => match key.code {
                KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                KeyCode::Backspace => {
                    weight_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = LogBodyweightField::Date
                }
                KeyCode::Up => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Esc => {
                    // Handle Esc directly here to avoid further processing
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Date => match key.code {
                KeyCode::Char(c) => date_input.push(c),
                KeyCode::Backspace => {
                    date_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = LogBodyweightField::Confirm
                }
                KeyCode::Up => *focused_field = LogBodyweightField::Weight,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Confirm => match key.code {
                KeyCode::Enter => {
                    // Prepare to submit *after* this block releases the borrow
                    should_submit = true;
                    weight_to_submit = weight_input.clone();
                    date_to_submit = date_input.clone();
                }
                KeyCode::Left | KeyCode::Backspace => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Up => *focused_field = LogBodyweightField::Date,
                KeyCode::Down | KeyCode::Tab => *focused_field = LogBodyweightField::Cancel,
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            LogBodyweightField::Cancel => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                KeyCode::Right => *focused_field = LogBodyweightField::Confirm,
                KeyCode::Up => *focused_field = LogBodyweightField::Date,
                KeyCode::Down | KeyCode::Tab => *focused_field = LogBodyweightField::Weight,
                _ => {}
            },
        }
    } // Mutable borrow of app.active_modal ends here

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        let submit_result = submit_log_bodyweight(app, &weight_to_submit, &date_to_submit);

        // Handle result: Re-borrow ONLY if necessary to set error
        if submit_result.is_ok() {
            app.active_modal = ActiveModal::None; // Submission successful, close modal
                                                  // Refresh handled by main loop
        } else {
            // Submission failed, need to put error back into modal state
            if let ActiveModal::LogBodyweight {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submit_result.unwrap_err().to_string());
                // Keep the modal open by not setting it to None
            }
            // If modal somehow changed state between submit check and here, error is lost, which is unlikely
        }
    }

    Ok(())
}
