// src/app/modals/edit_workout.rs

use crate::app::state::{ActiveModal, AddWorkoutField, App}; // Reuse AddWorkoutField enum
use crate::app::utils::{modify_numeric_input, parse_optional_float, parse_optional_int};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use task_athlete_lib::EditWorkoutParams;

// --- Submission Logic ---

fn submit_edit_workout(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::EditWorkout {
         workout_id,
         // exercise_name is not submitted for change here
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         resolved_exercise, // Needed for type context (bodyweight)
         .. // ignore focused_field, error_message
     } = modal_state {
        let mut edit_params = EditWorkoutParams::default();
        edit_params.id = i64::try_from(*workout_id)
            .map_err(|e| AppInputError::InvalidNumber(format!("{e}")))?;
        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             AppInputError::DbError("Internal error: Exercise context missing for edit.".to_string())
        })?;
        edit_params.new_exercise_identifier = Some(exercise_def.name.clone());
        // TODO exercise def

        // Parse inputs (reuse existing helpers)
        edit_params.new_sets = parse_optional_int(sets_input)?;
        edit_params.new_reps = parse_optional_int(reps_input)?;
        edit_params.new_weight = parse_optional_float(weight_input)?;
        edit_params.new_duration = parse_optional_int::<i64>(duration_input)?;
        edit_params.new_distance_arg = parse_optional_float(distance_input)?;
        edit_params.new_notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // Bodyweight & Units handled by service layer (though not passed explicitly here,
        // the service knows the type from the workout_id)
        // The original code passed it, but edit_workout in the library doesn't take it directly.
        // Let's stick to the library's signature.

        // Call AppService's edit_workout (assuming its signature)
        match app.service.edit_workout(
            edit_params
        ) {
            Ok(_) => Ok(()), // Success
            Err(e) => {
                Err(AppInputError::DbError(format!("Error editing workout: {e }")))
            }
        }
    } else {
        Err(AppInputError::DbError("Internal error: Invalid modal state for edit workout".to_string()))
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_edit_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;

    if let ActiveModal::EditWorkout {
        // workout_id and exercise_name are not directly modified by input
        ref mut sets_input,
        ref mut reps_input,
        ref mut weight_input,
        ref mut duration_input,
        ref mut distance_input,
        ref mut notes_input,
        ref mut focused_field,
        ref mut error_message,
        // resolved_exercise is needed for context but not directly edited
        ..
    } = app.active_modal
    {
        *error_message = None; // Clear error on input

        // Handle Shift+Tab for reverse navigation (simplified for edit modal)
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            match *focused_field {
                AddWorkoutField::Sets => *focused_field = AddWorkoutField::Cancel, // Wrap around up
                AddWorkoutField::Reps => *focused_field = AddWorkoutField::Sets,
                AddWorkoutField::Weight => *focused_field = AddWorkoutField::Reps,
                AddWorkoutField::Duration => *focused_field = AddWorkoutField::Weight,
                AddWorkoutField::Distance => *focused_field = AddWorkoutField::Duration,
                AddWorkoutField::Notes => *focused_field = AddWorkoutField::Distance,
                AddWorkoutField::Confirm => *focused_field = AddWorkoutField::Notes,
                AddWorkoutField::Cancel => *focused_field = AddWorkoutField::Confirm,
                _ => {} // Ignore Exercise/Suggestions fields which are not present/focusable in Edit
            }
        } else {
            // --- Handle other fields (Sets, Reps, etc.) ---
            // Reuse AddWorkoutField enum, but skip Exercise/Suggestions focus states
            match *focused_field {
                // Skip Exercise and Suggestions fields (they are not focusable in EditModal)
                AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                    // This should not happen if focus is managed correctly
                    // Default to moving to the first editable field
                    *focused_field = AddWorkoutField::Sets;
                }
                AddWorkoutField::Sets => {
                    match key.code {
                        KeyCode::Char(c) if c.is_ascii_digit() => sets_input.push(c),
                        KeyCode::Backspace => {
                            sets_input.pop();
                        }
                        KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                        KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                        KeyCode::Enter | KeyCode::Tab => {
                            *focused_field = AddWorkoutField::Reps;
                        }
                        KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Cancel; // Wrap around up
                        }
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Cancel; // Simple Up goes to Cancel
                        }
                        KeyCode::Down => {
                            *focused_field = AddWorkoutField::Reps;
                        } // Simple Down goes forward
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Reps => match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => reps_input.push(c),
                    KeyCode::Backspace => {
                        reps_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Sets;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Sets;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Weight => match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                    KeyCode::Backspace => {
                        weight_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true),
                    KeyCode::Down => {
                        modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Reps;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Reps;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Duration => match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => duration_input.push(c),
                    KeyCode::Backspace => {
                        duration_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Weight;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Distance => match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                    KeyCode::Backspace => {
                        distance_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true),
                    KeyCode::Down => {
                        modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Notes;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Duration;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Notes;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Notes => match key.code {
                    KeyCode::Char(c) => notes_input.push(c),
                    KeyCode::Backspace => {
                        notes_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Confirm;
                    }
                    KeyCode::BackTab => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Distance;
                    }
                    KeyCode::Down => {
                        *focused_field = AddWorkoutField::Confirm;
                    } // Go down to Confirm
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Confirm => {
                    match key.code {
                        KeyCode::Enter => should_submit = true,
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Cancel;
                        }
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Notes;
                        }
                        KeyCode::Down | KeyCode::Tab | KeyCode::Right => {
                            *focused_field = AddWorkoutField::Cancel;
                        } // Wrap around
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Cancel => {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Tab => {
                            *focused_field = AddWorkoutField::Confirm;
                        }
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            *focused_field = AddWorkoutField::Confirm;
                        }
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Notes;
                        }
                        KeyCode::Down => {
                            *focused_field = AddWorkoutField::Sets;
                        } // Wrap around down to Sets
                        _ => {}
                    }
                }
            }
        }
    } // End mutable borrow of app.active_modal

    // --- Submission Logic ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::EditWorkout { .. } = modal_state_clone {
            submission_result = submit_edit_workout(app, &modal_state_clone);
        } else {
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly".to_string(),
            ));
        }

        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
        } else {
            // Re-borrow to set error
            if let ActiveModal::EditWorkout {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}
