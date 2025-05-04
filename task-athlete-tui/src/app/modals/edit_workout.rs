// src/app/modals/edit_workout.rs
use super::input_helpers::{get_next_focusable_field, NavigationDirection}; // Import helper
use crate::app::state::{ActiveModal, AddWorkoutField, App, WorkoutLogFlags}; // Import WorkoutLogFlags
use crate::app::utils::{modify_numeric_input, parse_optional_float, parse_optional_int};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers}; // Keep KeyModifiers
use task_athlete_lib::EditWorkoutParams;

// --- Submission Logic ---
// submit_edit_workout function remains the same as the corrected version from the previous step
fn submit_edit_workout(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::EditWorkout {
        workout_id,
        sets_input,
        reps_input,
        weight_input,
        duration_input,
        distance_input,
        notes_input,
        resolved_exercise, // Needed for type context and flags
        ..
    } = modal_state
    {
        let mut edit_params = EditWorkoutParams::default();
        edit_params.id =
            i64::try_from(*workout_id).map_err(|e| AppInputError::InvalidNumber(format!("{e}")))?;

        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
            AppInputError::DbError("Internal error: Exercise context missing for edit.".to_string())
        })?;
        // We don't submit exercise identifier change here, but we use the def for flags
        edit_params.new_exercise_identifier = None; // Exercise name not editable here

        // Parse inputs conditionally based on flags
        let flags = WorkoutLogFlags::from_def(Some(exercise_def));
        edit_params.new_sets = if flags.log_sets {
            parse_optional_int(sets_input)?
        } else {
            None
        };
        edit_params.new_reps = if flags.log_reps {
            parse_optional_int(reps_input)?
        } else {
            None
        };
        edit_params.new_weight = if flags.log_weight {
            parse_optional_float(weight_input)?
        } else {
            None
        };
        edit_params.new_duration = if flags.log_duration {
            parse_optional_int::<i64>(duration_input)?
        } else {
            None
        };
        edit_params.new_distance_arg = if flags.log_distance {
            parse_optional_float(distance_input)?
        } else {
            None
        };
        edit_params.new_notes = if flags.log_notes && !notes_input.trim().is_empty() {
            Some(notes_input.trim().to_string())
        } else {
            None
        };

        // Call AppService's edit_workout
        match app.service.edit_workout(edit_params) {
            Ok(_) => Ok(()), // Success
            Err(e) => Err(AppInputError::DbError(format!(
                "Error editing workout: {e }"
            ))),
        }
    } else {
        Err(AppInputError::DbError(
            "Internal error: Invalid modal state for edit workout".to_string(),
        ))
    }
}

// --- Input Handling ---

pub fn handle_edit_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;

    // --- Get Flags and Current Focus Early ---
    let (current_focused_field, flags, is_add_mode) = {
        // Use immutable borrow first
        if let ActiveModal::EditWorkout {
            focused_field,
            resolved_exercise,
            ..
        } = &app.active_modal
        {
            (
                *focused_field,
                WorkoutLogFlags::from_def(resolved_exercise.as_ref()),
                false,
            ) // is_add_mode is false
        } else {
            (AddWorkoutField::Cancel, WorkoutLogFlags::default(), false)
        }
    };

    // --- Main Input Handling Logic (Inside mutable borrow) ---
    if let ActiveModal::EditWorkout {
        // Use `ref mut` for mutable fields
        ref mut sets_input, ref mut reps_input, ref mut weight_input,
        ref mut duration_input, ref mut distance_input, ref mut notes_input,
        ref mut focused_field, ref mut error_message, .. // Ignore others here
    } = app.active_modal // Get mutable references here
    {
        *error_message = None; // Clear error at the beginning
        let mut focus_changed = false;

        // Local helper to update focus using the mutable ref obtained above
        let current_focus = *focused_field;
        let mut move_focus = |direction: NavigationDirection| {
            *focused_field = get_next_focusable_field(current_focused_field, &flags, direction, is_add_mode);
            focus_changed = true;
        };

        // Handle inputs using the focus helper
        // Handle Shift+Tab for reverse navigation
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            move_focus(NavigationDirection::Backward);
        } else {
            match current_focus { // Match on the ref mut field directly
                // Skip Exercise and Suggestions - shouldn't be focusable in Edit mode
                AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                    // Fallback: move to the first editable field
                    move_focus(NavigationDirection::Forward); // Will likely go to Sets
                }
                AddWorkoutField::Sets => {
                    match key.code {
                        KeyCode::Char(c) if c.is_ascii_digit() => sets_input.push(c),
                        KeyCode::Backspace => { sets_input.pop(); }
                        KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                        KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                        KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                        // BackTab handled above
                        // Removed Up/Down direct focus change
                        KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                        _ => {}
                    }
                }
                AddWorkoutField::Reps => match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => reps_input.push(c),
                    KeyCode::Backspace => { reps_input.pop(); }
                    KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                    // BackTab handled above
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                },
                 AddWorkoutField::Weight => match key.code {
                     KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                     KeyCode::Backspace => { weight_input.pop(); }
                     KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true),
                     KeyCode::Down => modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true),
                     KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                     // BackTab handled above
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                },
                AddWorkoutField::Duration => match key.code {
                     KeyCode::Char(c) if c.is_ascii_digit() => duration_input.push(c),
                     KeyCode::Backspace => { duration_input.pop(); }
                     KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false),
                     KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                     KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                     // BackTab handled above
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                },
                AddWorkoutField::Distance => match key.code {
                     KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                     KeyCode::Backspace => { distance_input.pop(); }
                     KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true),
                     KeyCode::Down => modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true),
                     KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                     // BackTab handled above
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                },
                AddWorkoutField::Notes => match key.code {
                     KeyCode::Char(c) => notes_input.push(c),
                     KeyCode::Backspace => { notes_input.pop(); }
                     KeyCode::Enter | KeyCode::Tab => move_focus(NavigationDirection::Forward), // Enter behaves like Tab
                     // BackTab handled above
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                },
                AddWorkoutField::Confirm => match key.code {
                     KeyCode::Enter => should_submit = true,
                     KeyCode::Left | KeyCode::Backspace => move_focus(NavigationDirection::Backward),
                     KeyCode::Up => move_focus(NavigationDirection::Backward),
                     KeyCode::Down | KeyCode::Tab | KeyCode::Right => move_focus(NavigationDirection::Forward),
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                },
                AddWorkoutField::Cancel => match key.code {
                     KeyCode::Enter | KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     KeyCode::Right | KeyCode::Tab => move_focus(NavigationDirection::Forward),
                     KeyCode::Left | KeyCode::Backspace => move_focus(NavigationDirection::Backward),
                     KeyCode::Up => move_focus(NavigationDirection::Backward),
                     KeyCode::Down => move_focus(NavigationDirection::Forward),
                     _ => {}
                },
            }
        }
    } // End mutable borrow

    // --- Submission Logic (outside borrow) ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone(); // Clone before immutable borrow
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
                ref mut error_message, // Get mutable ref again
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
            }
        }
    }

    Ok(())
}
