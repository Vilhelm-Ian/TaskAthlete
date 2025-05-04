// src/app/modals/create_exercise.rs

use crate::app::state::{ActiveModal, AddExerciseField, App};
use crate::app::AppInputError;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use task_athlete_lib::{DbError, ExerciseType};

// --- Submission Logic ---

fn submit_create_exercise(app: &App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
        log_weight,
        log_reps,
        log_duration,
        log_distance,
        // ignore focused_field, error_message
        ..
    } = modal_state
    {
        let trimmed_name = name_input.trim();
        if trimmed_name.is_empty() {
            return Err(AppInputError::ExerciseNameEmpty);
        }

        let muscles_opt = if muscles_input.trim().is_empty() {
            None
        } else {
            Some(muscles_input.trim())
        };

        let arguments = convert_flags(*log_duration, *log_distance, *log_weight, *log_reps);

        // Call AppService to create the exercise
        match app
            .service
            .create_exercise(trimmed_name, *selected_type, arguments, muscles_opt)
        {
            Ok(_) => Ok(()), // Signal success to close modal
            Err(e) => {
                // Convert service error to modal error
                if let Some(db_err) = e.downcast_ref::<DbError>() {
                    // Handle specific unique constraint error
                    if let DbError::ExerciseNameNotUnique(name) = db_err {
                        return Err(AppInputError::DbError(format!(
                            "Exercise '{}' already exists.",
                            name
                        )));
                    }
                    Err(AppInputError::DbError(db_err.to_string()))
                } else {
                    Err(AppInputError::DbError(format!(
                        "Error creating exercise: {}",
                        e
                    )))
                }
            }
        }
    } else {
        // Should not happen if called correctly
        Err(AppInputError::DbError(
            "Internal error: Invalid modal state for create exercise".to_string(),
        ))
    }
}

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_create_exercise_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;
    // let mut focus_changed = false; // Not strictly needed for this modal handling logic

    if let ActiveModal::CreateExercise {
        ref mut name_input,
        ref mut muscles_input,
        ref mut selected_type,
        ref mut focused_field,
        ref mut error_message,
        ref mut log_weight,
        ref mut log_reps,
        ref mut log_duration,
        ref mut log_distance,
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;

        // Handle Shift+Tab for reverse navigation
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            match *focused_field {
                AddExerciseField::Name => *focused_field = AddExerciseField::Cancel,
                AddExerciseField::Muscles => *focused_field = AddExerciseField::Name,
                AddExerciseField::TypeResistance => *focused_field = AddExerciseField::Muscles,
                AddExerciseField::TypeCardio => *focused_field = AddExerciseField::TypeResistance,
                AddExerciseField::TypeBodyweight => *focused_field = AddExerciseField::TypeCardio,
                AddExerciseField::LogWeight => *focused_field = AddExerciseField::TypeBodyweight,
                AddExerciseField::LogReps => *focused_field = AddExerciseField::LogWeight,
                AddExerciseField::LogDuration => *focused_field = AddExerciseField::LogReps,
                AddExerciseField::LogDistance => *focused_field = AddExerciseField::LogDuration,
                AddExerciseField::Confirm => *focused_field = AddExerciseField::TypeBodyweight,
                AddExerciseField::Cancel => *focused_field = AddExerciseField::Confirm,
            }
            // focus_changed = true;
        } else {
            // Handle normal key presses
            match *focused_field {
                AddExerciseField::Name => match key.code {
                    KeyCode::Char(c) => name_input.push(c),
                    KeyCode::Backspace => {
                        name_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddExerciseField::Muscles;
                        // focus_changed = true;
                    }
                    KeyCode::Up => *focused_field = AddExerciseField::Cancel, // Wrap around up
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::Muscles => match key.code {
                    KeyCode::Char(c) => muscles_input.push(c),
                    KeyCode::Backspace => {
                        muscles_input.pop();
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddExerciseField::TypeResistance; // Move to first type
                                                                           // focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Name;
                        // focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Type Selection Fields ---
                // Enter confirms selection, Right/Tab/Down moves to next type, Left wraps or moves back
                AddExerciseField::TypeResistance => match key.code {
                    KeyCode::Enter => {
                        *selected_type = ExerciseType::Resistance;
                        *log_weight = true;
                        *log_reps = true;
                        *log_duration = false;
                        *log_distance = false;
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        *focused_field = AddExerciseField::TypeCardio;
                        // focus_changed = true;
                    }
                    KeyCode::Down => *focused_field = AddExerciseField::LogWeight,
                    KeyCode::Left | KeyCode::BackTab => {
                        // Also allow BackTab to go to Muscles
                        *focused_field = AddExerciseField::Muscles;
                        // focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles;
                        // focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeCardio => match key.code {
                    KeyCode::Enter => {
                        *selected_type = ExerciseType::Cardio;
                        *log_weight = false;
                        *log_reps = false;
                        *log_duration = true;
                        *log_distance = true;
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        *focused_field = AddExerciseField::TypeBodyweight;
                        // focus_changed = true;
                    }
                    KeyCode::Down => *focused_field = AddExerciseField::LogWeight,
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::TypeResistance;
                        // focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Stay within types or jump back? Let's jump back to Resistance for consistency with Down
                        *focused_field = AddExerciseField::TypeResistance;
                        // focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeBodyweight => match key.code {
                    KeyCode::Enter => {
                        *selected_type = ExerciseType::BodyWeight;
                        *log_weight = false;
                        *log_reps = true;
                        *log_duration = false;
                        *log_distance = false;
                    }
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::LogWeight; // Move to confirm
                                                                      // focus_changed = true;
                    }
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::TypeCardio;
                        // focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Stay within types or jump back? Let's jump back to Cardio
                        *focused_field = AddExerciseField::TypeCardio;
                        // focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Logging Flags Checkboxes ---
                AddExerciseField::LogWeight => match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => *log_weight = !*log_weight,
                    KeyCode::Right | KeyCode::Tab => *focused_field = AddExerciseField::LogReps,
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::TypeBodyweight
                    } // Wrap around or go back up
                    KeyCode::Down => *focused_field = AddExerciseField::Confirm, // Jump down to Confirm
                    KeyCode::Up => *focused_field = AddExerciseField::TypeBodyweight, // Jump up to last type
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::LogReps => match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => *log_reps = !*log_reps,
                    KeyCode::Right | KeyCode::Tab => *focused_field = AddExerciseField::LogDuration,
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::LogWeight
                    }
                    KeyCode::Down => *focused_field = AddExerciseField::Confirm, // Jump down to Confirm
                    KeyCode::Up => *focused_field = AddExerciseField::TypeBodyweight, // Jump up to last type
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::LogDuration => match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => *log_duration = !*log_duration,
                    KeyCode::Right | KeyCode::Tab => *focused_field = AddExerciseField::LogDistance,
                    KeyCode::Left | KeyCode::BackTab => *focused_field = AddExerciseField::LogReps,
                    KeyCode::Down => *focused_field = AddExerciseField::Confirm, // Jump down to Confirm
                    KeyCode::Up => *focused_field = AddExerciseField::TypeBodyweight, // Jump up to last type
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::LogDistance => match key.code {
                    KeyCode::Enter | KeyCode::Char(' ') => *log_distance = !*log_distance,
                    KeyCode::Right | KeyCode::Tab => *focused_field = AddExerciseField::Confirm, // Wrap around to Confirm
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::LogDuration
                    }
                    KeyCode::Down => *focused_field = AddExerciseField::Confirm, // Jump down to Confirm
                    KeyCode::Up => *focused_field = AddExerciseField::TypeBodyweight, // Jump up to last type
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Button Fields ---
                AddExerciseField::Confirm => match key.code {
                    KeyCode::Enter => {
                        should_submit = true;
                    }
                    KeyCode::Left | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::Cancel;
                        // focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the last type field
                        *focused_field = AddExerciseField::LogDistance;
                        // focus_changed = true;
                    }
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Cancel; // Cycle behavior
                                                                   // focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::Cancel => match key.code {
                    KeyCode::Enter | KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        *focused_field = AddExerciseField::Confirm;
                        // focus_changed = true;
                    }
                    KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                        *focused_field = AddExerciseField::Confirm; // Cycle behavior
                                                                    // focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the last type field
                        *focused_field = AddExerciseField::TypeBodyweight;
                        // focus_changed = true;
                    }
                    KeyCode::Down => {
                        *focused_field = AddExerciseField::Name; // Wrap around to top
                                                                 // focus_changed = true;
                    }
                    _ => {}
                },
            }
        }
    } // End mutable borrow of app.active_modal

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::CreateExercise { .. } = modal_state_clone {
            submission_result = submit_create_exercise(app, &modal_state_clone);
        // Pass the clone
        } else {
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly".to_string(),
            ));
        }

        // --- Handle Submission Result ---
        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Refresh handled by main loop
        } else {
            // Submission failed, re-borrow mutably ONLY if necessary to set error
            if let ActiveModal::CreateExercise {
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

fn convert_flags(
    weight: bool,
    reps: bool,
    duration: bool,
    distance: bool,
) -> Option<(Option<bool>, Option<bool>, Option<bool>, Option<bool>)> {
    match (duration, distance, weight, reps) {
        (false, false, false, false) => None,
        _ => Some((Some(duration), Some(distance), Some(weight), Some(reps))),
    }
}
