use super::state::{
    ActiveModal, AddExerciseField, AddWorkoutField, App, LogBodyweightField, SetTargetWeightField,
};
use super::AppInputError;
use anyhow::Result;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::str::FromStr;
use task_athlete_lib::{DbError, ExerciseDefinition, ExerciseType};

// --- Parsing Helpers (moved here) ---

fn parse_optional_int<T: FromStr>(input: &str) -> Result<Option<T>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<T>()
            .map(Some)
            .map_err(|_| {
                AppInputError::InvalidNumber(format!("'{trimmed}' is not a valid integer"))
            })
            .inspect(|opt_val| {
                // Basic validation (can be extended)
                if let Some(val) = opt_val.as_ref() {
                    // Assuming T supports comparison with 0 (like i64)
                    // This requires a bound, maybe add later if T is generic
                    // if *val < 0 { return Err(AppInputError::InvalidNumber("Value cannot be negative".into())) }
                }
            })
    }
}

fn parse_optional_float(input: &str) -> Result<Option<f64>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<f64>()
            .map(Some)
            .map_err(|_| AppInputError::InvalidNumber(format!("'{trimmed}' is not a valid number")))
            .and_then(|opt_val| {
                if let Some(val) = opt_val {
                    if val < 0.0 {
                        return Err(AppInputError::InvalidNumber(
                            "Value cannot be negative".into(),
                        ));
                    }
                }
                Ok(opt_val)
            })
    }
}

// Helper to increment/decrement a numeric string field
fn modify_numeric_input<T>(input_str: &mut String, delta: T, min_val: Option<T>, is_float: bool)
where
    T: FromStr
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy
        + std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Debug,
{
    // let current_val = if is_float {
    //     input_str.parse::<f64>().ok()
    // } else {
    //     input_str.parse::<i64>().ok()
    // };

    let mut num_val: T = match input_str.parse::<T>() {
        Ok(v) => v,
        Err(_) => return, // Cannot parse, do nothing
    };

    num_val = num_val + delta; // Apply delta

    // Apply minimum value constraint
    if let Some(min) = min_val {
        if num_val < min {
            num_val = min;
        }
    }

    // Update the string
    if is_float {
        *input_str = format!("{:.1}", num_val); // Format floats nicely
    } else {
        *input_str = num_val.to_string();
    }
}

fn parse_modal_date(date_str: &str) -> Result<NaiveDate, AppInputError> {
    let trimmed = date_str.trim().to_lowercase();
    match trimmed.as_str() {
        "today" => Ok(Utc::now().date_naive()),
        "yesterday" => Ok(Utc::now().date_naive() - Duration::days(1)),
        _ => NaiveDate::parse_from_str(&trimmed, "%Y-%m-%d")
            .map_err(|_| AppInputError::InvalidDate(date_str.to_string())),
    }
}

fn parse_modal_weight(weight_str: &str) -> Result<f64, AppInputError> {
    let trimmed = weight_str.trim();
    if trimmed.is_empty() {
        return Err(AppInputError::InputEmpty);
    }
    trimmed
        .parse::<f64>()
        .map_err(|e| AppInputError::InvalidNumber(e.to_string()))
        .and_then(|w| {
            if w > 0.0 {
                Ok(w)
            } else {
                Err(AppInputError::InvalidNumber(
                    "Weight must be positive".to_string(),
                ))
            }
        })
}

// --- Submission Logic ---

fn submit_add_workout(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::AddWorkout {
         exercise_input: _, // Use resolved_exercise name
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         resolved_exercise, // Use the stored resolved exercise
         .. // ignore focused_field, error_message, suggestions etc.
     } = modal_state {

        // 1. Validate Exercise Selection
        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             // This error should ideally be prevented by the input handler (not allowing Tab/Enter without resolution)
             AppInputError::DbError("Exercise not resolved. Select a valid exercise.".to_string())
        })?;
        let canonical_name = &exercise_def.name; // Already resolved

        // 2. Parse numeric inputs
        let sets = parse_optional_int::<i64>(sets_input)?;
        let reps = parse_optional_int::<i64>(reps_input)?;
        let weight_arg = parse_optional_float(weight_input)?; // This is the value from the input field
        let duration = parse_optional_int::<i64>(duration_input)?;
        let distance_arg = parse_optional_float(distance_input)?; // Value from input field

        // 3. Notes
        let notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // 4. Bodyweight & Units (Service layer handles this based on type and config)
        let bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight // Pass the configured bodyweight
        } else {
            None
        };


        // 5. Call AppService
        match app.service.add_workout(
            canonical_name,
            app.log_viewed_date, // Use the date currently viewed in the log tab
            sets,
            reps,
            weight_arg, // Pass the weight from the input field
            duration,
            distance_arg, // Pass the distance from the input field
            notes,
            None, // No implicit type needed (already resolved)
            None, // No implicit muscles needed (already resolved)
            bodyweight_to_use, // Pass configured bodyweight if needed
        ) {
            Ok((_workout_id, pb_info)) => {
                 if let Some(pb) = pb_info {
                    // Simple message if any PB achieved
                    if pb.any_pb() {
                         // Using set_error might be confusing, maybe a different status method?
                         // For now, use set_error for feedback.
                         app.set_error("🎉 New Personal Best achieved!".to_string());
                    }
                 }
                Ok(()) // Signal success to close modal
            }
            Err(e) => {
                 // Convert service error to modal error
                 if let Some(db_err) = e.downcast_ref::<DbError>() {
                     Err(AppInputError::DbError(db_err.to_string()))
                 } else if let Some(cfg_err) = e.downcast_ref::<task_athlete_lib::ConfigError>() {
                      Err(AppInputError::DbError(cfg_err.to_string())) // Use DbError variant for simplicity
                 }
                 else {
                     Err(AppInputError::DbError(format!("Error adding workout: {}", e)))
                 }
            }
        }

     } else {
         // Should not happen if called correctly
         Err(AppInputError::DbError("Internal error: Invalid modal state for add workout".to_string()))
     }
}

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

        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             AppInputError::DbError("Internal error: Exercise context missing for edit.".to_string())
        })?;

        // Parse inputs (reuse existing helpers)
        let sets = parse_optional_int(sets_input)?;
        let reps = parse_optional_int(reps_input)?;
        let weight_arg = parse_optional_float(weight_input)?;
        let duration = parse_optional_int::<i64>(duration_input)?;
        let distance_arg = parse_optional_float(distance_input)?;
        let notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // Bodyweight & Units handled by service layer
        let bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight
        } else { None };

        // Call AppService's edit_workout (assuming its signature)
        // Adjust the signature call based on your actual AppService::edit_workout method
        match app.service.edit_workout(
            *workout_id as i64, None, sets, reps, weight_arg, duration, distance_arg, notes, None
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

fn submit_create_exercise(app: &App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
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

        // Call AppService to create the exercise
        match app
            .service
            .create_exercise(trimmed_name, *selected_type, muscles_opt)
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
                _ => {} // Ignore Exercise/Suggestions fields
            }
        } else {
            // --- Handle other fields (Sets, Reps, etc.) ---
            // Reuse AddWorkoutField enum, but skip Exercise/Suggestions focus states
            match *focused_field {
                // Skip Exercise and Suggestions fields
                AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                    *focused_field = AddWorkoutField::Sets; // Should not be focusable, move to Sets
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
                            *focused_field = AddWorkoutField::Cancel;
                        } // Defined above
                        KeyCode::Up => {
                            *focused_field = AddWorkoutField::Cancel;
                        } // Simple Up goes to Cancel
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
                    }
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

pub fn handle_confirm_delete_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut should_delete = false;
    let mut workout_id_to_delete: u64 = 0; // Placeholder

    if let ActiveModal::ConfirmDeleteWorkout { workout_id, .. } = &app.active_modal {
        workout_id_to_delete = *workout_id; // Capture the ID
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

pub fn handle_add_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;
    let mut needs_suggestion_update = false;
    // Flag to indicate that workout fields should be repopulated
    let mut repopulate_fields_for_resolved_exercise: Option<ExerciseDefinition> = None;

    if let ActiveModal::AddWorkout {
        ref mut exercise_input,
        ref mut sets_input,
        ref mut reps_input,
        ref mut weight_input,
        ref mut duration_input,
        ref mut distance_input,
        ref mut notes_input,
        ref mut focused_field,
        ref mut error_message,
        ref mut resolved_exercise,
        ref mut exercise_suggestions,
        ref mut suggestion_list_state,
        .. // ignore all_exercise_identifiers
    } = app.active_modal
    {
        *error_message = None; // Clear error on most inputs
        let mut focus_changed = false;

        // --- Main Input Handling Logic ---
        match *focused_field {
            AddWorkoutField::Exercise => match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None; // Invalidate resolution
                    needs_suggestion_update = true; // Filter suggestions after borrow
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None; // Invalidate resolution
                    needs_suggestion_update = true; // Filter suggestions after borrow
                }
                KeyCode::Down => {
                    if !exercise_suggestions.is_empty() {
                        *focused_field = AddWorkoutField::Suggestions;
                        suggestion_list_state.select(Some(0));
                        focus_changed = true;
                    } else {
                        // No suggestions, behave like Tab (go to Sets)
                        // Attempt to resolve before moving
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone();
                                if resolved_exercise.as_ref() != Some(&def) { // Check if it changed
                                    repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets;
                                focus_changed = true;
                                *exercise_suggestions = Vec::new(); // Clear suggestions
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => {
                                *error_message = Some(format!("Exercise '{}' not found.", exercise_input));
                                // Optionally clear fields if resolution fails? Maybe not.
                            }
                            Err(e) => *error_message = Some(format!("Error: {}", e)),
                        }
                    }
                }
                 KeyCode::Tab => {
                    // Attempt to resolve current input before moving
                    if resolved_exercise.is_none() && !exercise_input.is_empty() {
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone(); // Update input to canonical name
                                if resolved_exercise.as_ref() != Some(&def) { // Check if it *really* changed
                                    repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets;
                                focus_changed = true;
                                *exercise_suggestions = Vec::new(); // Clear suggestions after resolving/moving away
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => {
                                *error_message = Some(format!("Exercise '{}' not found. Cannot move.", exercise_input));
                                // Optionally clear fields if resolution fails? Maybe not.
                            } // Stay if not resolved
                            Err(e) => {
                                *error_message = Some(format!("Error resolving: {}. Cannot move.", e));
                            } // Stay if error
                        }
                    } else { // Move if already resolved or empty
                        *focused_field = AddWorkoutField::Sets;
                        focus_changed = true;
                        *exercise_suggestions = Vec::new(); // Clear suggestions
                        suggestion_list_state.select(None);
                    }
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Cancel;
                    focus_changed = true;
                    *exercise_suggestions = Vec::new();
                    suggestion_list_state.select(None);
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },

            AddWorkoutField::Suggestions => match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                KeyCode::Up => {
                     if !exercise_suggestions.is_empty() {
                        let current_selection = suggestion_list_state.selected().unwrap_or(0);
                        let new_selection = if current_selection == 0 {
                            exercise_suggestions.len() - 1
                        } else {
                            current_selection - 1
                        };
                        suggestion_list_state.select(Some(new_selection));
                    }
                }
                KeyCode::Down => {
                    if !exercise_suggestions.is_empty() {
                        let current_selection = suggestion_list_state.selected().unwrap_or(0);
                        let new_selection = if current_selection >= exercise_suggestions.len() - 1 {
                            0
                        } else {
                            current_selection + 1
                        };
                        suggestion_list_state.select(Some(new_selection));
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected_index) = suggestion_list_state.selected() {
                        if let Some(selected_suggestion) = exercise_suggestions.get(selected_index) {
                            match app.service.resolve_exercise_identifier(selected_suggestion) {
                                Ok(Some(def)) => {
                                    *exercise_input = def.name.clone();
                                    if resolved_exercise.as_ref() != Some(&def) { // Check if it changed
                                        repopulate_fields_for_resolved_exercise = Some(def.clone());
                                    }
                                    *resolved_exercise = Some(def);
                                    *focused_field = AddWorkoutField::Sets;
                                    focus_changed = true;
                                    *exercise_suggestions = Vec::new(); // Clear suggestions after selection
                                    suggestion_list_state.select(None);
                                }
                                Ok(None) => {
                                    *error_message = Some(format!("Could not resolve selected '{}'.", selected_suggestion));
                                    *focused_field = AddWorkoutField::Exercise;
                                    focus_changed = true;
                                    // Do not clear suggestions if resolution failed
                                }
                                Err(e) => {
                                    *error_message = Some(format!("Error resolving selected: {}", e));
                                    *focused_field = AddWorkoutField::Exercise;
                                    focus_changed = true;
                                    // Do not clear suggestions if resolution failed
                                }
                            }
                        }
                    } else {
                         // If somehow Enter hit with no selection, try resolving current input
                        match app.service.resolve_exercise_identifier(exercise_input) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone();
                                if resolved_exercise.as_ref() != Some(&def) {
                                     repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                *focused_field = AddWorkoutField::Sets; // Move to next field
                                focus_changed = true;
                                *exercise_suggestions = Vec::new();
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => { // Enter pressed but input not resolvable -> back to input
                                *focused_field = AddWorkoutField::Exercise;
                                focus_changed = true;
                            }
                             Err(e) => { // Error resolving -> back to input
                                *error_message = Some(format!("Error resolving input: {}", e));
                                *focused_field = AddWorkoutField::Exercise;
                                focus_changed = true;
                            }
                        }
                    }
                }
                KeyCode::Tab | KeyCode::Esc => {
                    // Exit suggestion list back to input field
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                    // Keep suggestions visible for now when going back via Esc/Tab
                }
                _ => {}
            },

             // --- Handle other fields (Sets, Reps, etc.) ---
             // Common pattern: Clear suggestions and move focus
             AddWorkoutField::Sets => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None); // Clear suggestions
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => sets_input.push(c),
                    KeyCode::Backspace => { sets_input.pop(); }
                    KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                    KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    // Shift+Tab for reverse navigation
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; } // Simple Up goes back
                    KeyCode::Down => { *focused_field = AddWorkoutField::Reps; focus_changed = true; } // Simple Down goes forward
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Reps => {
                 *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                 match key.code {
                     KeyCode::Char(c) if c.is_ascii_digit() => reps_input.push(c),
                     KeyCode::Backspace => { reps_input.pop(); }
                     KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                     KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                     KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                     KeyCode::BackTab => { *focused_field = AddWorkoutField::Sets; focus_changed = true; }
                     KeyCode::Up => { *focused_field = AddWorkoutField::Sets; focus_changed = true; }
                     KeyCode::Down => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                     KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                     _ => {}
                 }
            }
            AddWorkoutField::Weight => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                    KeyCode::Backspace => { weight_input.pop(); }
                    KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true),
                    KeyCode::Down => modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Reps; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Duration => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => duration_input.push(c),
                    KeyCode::Backspace => { duration_input.pop(); }
                    KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false),
                    KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Weight; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Distance => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                    KeyCode::Backspace => { distance_input.pop(); }
                    KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true),
                    KeyCode::Down => modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true),
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::BackTab => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Duration; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Notes => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Char(c) => notes_input.push(c),
                    KeyCode::Backspace => { notes_input.pop(); }
                    // Treat Enter like Tab for Notes
                    KeyCode::Enter | KeyCode::Tab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; }
                     KeyCode::BackTab => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Distance; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; } // Go down to Confirm
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Confirm => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Enter => {
                        // Final validation before submit?
                         if resolved_exercise.is_none() {
                            *error_message = Some("Cannot submit: Exercise not resolved.".to_string());
                            *focused_field = AddWorkoutField::Exercise; // Send user back to fix it
                         } else {
                            should_submit = true;
                         }
                    }
                    KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => { *focused_field = AddWorkoutField::Cancel; focus_changed = true; }
                    KeyCode::Up => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Down | KeyCode::Tab | KeyCode::Right => { *focused_field = AddWorkoutField::Cancel; focus_changed = true; } // Wrap around
                    KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    _ => {}
                }
            }
            AddWorkoutField::Cancel => {
                *exercise_suggestions = Vec::new(); suggestion_list_state.select(None);
                match key.code {
                    KeyCode::Enter | KeyCode::Esc => { app.active_modal = ActiveModal::None; return Ok(()); }
                    KeyCode::Right | KeyCode::Tab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; }
                    KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => { *focused_field = AddWorkoutField::Confirm; focus_changed = true; } // Cycle left from Cancel goes to Confirm
                    KeyCode::Up => { *focused_field = AddWorkoutField::Notes; focus_changed = true; }
                    KeyCode::Down => { *focused_field = AddWorkoutField::Exercise; focus_changed = true; } // Wrap around down
                    _ => {}
                }
            }
        }

        // If focus moved away from Exercise field and it wasn't resolved, try to resolve now.
        // This is a fallback, main resolution happens on Tab/Enter/Suggestion select.
        if focus_changed
            && *focused_field != AddWorkoutField::Exercise
            && *focused_field != AddWorkoutField::Suggestions
            && resolved_exercise.is_none()
            && !exercise_input.is_empty() // Only try if there's input
        {
            match app.service.resolve_exercise_identifier(exercise_input) {
                Ok(Some(def)) => {
                    *exercise_input = def.name.clone(); // Update input field too
                    if resolved_exercise.as_ref() != Some(&def) { // Check if changed
                         repopulate_fields_for_resolved_exercise = Some(def.clone());
                    }
                    *resolved_exercise = Some(def);
                }
                 Ok(None) => {
                     // Allow moving away, but show error if resolution failed?
                     // Or maybe just clear the resolved_exercise state?
                     // Let's clear it and maybe show a warning if they try to submit.
                     *resolved_exercise = None;
                     // Optional: *error_message = Some(format!("Warning: Exercise '{}' not resolved.", exercise_input));
                 }
                Err(e) => {
                    *resolved_exercise = None; // Clear on error too
                    *error_message = Some(format!("Error resolving '{}': {}", exercise_input, e));
                }
            }
        }

    } // End mutable borrow of app.active_modal

    // --- Repopulate Fields (Deferred until borrow ends) ---
    if let Some(def_to_repopulate) = repopulate_fields_for_resolved_exercise {
        // Re-borrow mutably to update fields
        let last_workout = app.get_last_or_specific_workout(&def_to_repopulate.name, None);
        if let ActiveModal::AddWorkout {
            ref mut sets_input,
            ref mut reps_input,
            ref mut weight_input,
            ref mut duration_input,
            ref mut distance_input,
            // notes_input not typically repopulated
            ..
        } = app.active_modal
        {
            if let Some(workout) = last_workout {
                *sets_input = parse_option_to_input(workout.sets);
                *reps_input = parse_option_to_input(workout.reps);
                *weight_input = parse_option_to_input(workout.weight);
                *distance_input = parse_option_to_input(workout.distance);
                *duration_input = parse_option_to_input(workout.duration_minutes);
            }
        }
    }

    // --- Filter suggestions (Deferred until borrow ends) ---
    if needs_suggestion_update {
        app.filter_exercise_suggestions();
    }

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        // Clone the state *before* calling submit, as submit needs immutable borrow
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::AddWorkout { .. } = modal_state_clone {
            submission_result = submit_add_workout(app, &modal_state_clone);
        } else {
            // This case should be rare due to the check within the Confirm handler
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly before submit".to_string(),
            ));
        }

        // --- Handle Submission Result ---
        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Data refresh will happen in the main loop
        } else {
            // Re-borrow mutably ONLY if necessary to set error
            if let ActiveModal::AddWorkout {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
                // Optionally, set focus back to a relevant field? e.g., Exercise if that was the issue?
                // Or just keep focus where it was (Confirm button).
            }
        }
    }

    Ok(())
}

pub fn handle_log_bodyweight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut date_to_submit = String::new();
    let mut should_submit = false;
    let mut focus_after_input = LogBodyweightField::Weight; // Default

    if let ActiveModal::LogBodyweight {
        ref mut weight_input,
        ref mut date_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;
        focus_after_input = *focused_field; // Store current focus

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

pub fn handle_set_target_weight_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    // Temporary storage for data if we need to call submit_*
    let mut weight_to_submit = String::new();
    let mut submit_action: Option<fn(&mut App, &str) -> Result<(), AppInputError>> = None; // For Set
    let mut clear_action: Option<fn(&mut App) -> Result<(), AppInputError>> = None; // For Clear
    let mut focus_after_input = SetTargetWeightField::Weight; // Default

    if let ActiveModal::SetTargetWeight {
        ref mut weight_input,
        ref mut focused_field,
        ref mut error_message,
    } = app.active_modal
    {
        *error_message = None; // Clear error on any input
        focus_after_input = *focused_field;

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
                KeyCode::Tab => *focused_field = SetTargetWeightField::Weight,
                KeyCode::Up => *focused_field = SetTargetWeightField::Clear,
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

pub fn handle_confirm_delete_body_weigth_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut should_delete = false;
    let mut bodyweight_id_to_delete: u64 = 0; // Placeholder

    if let ActiveModal::ConfirmDeleteBodyWeight { body_weight_id, .. } = &app.active_modal {
        bodyweight_id_to_delete = *body_weight_id; // Capture the ID
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
    }

    if should_delete {
        let delete_result = sumbit_delete_body_weight(app, bodyweight_id_to_delete);
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

fn sumbit_delete_body_weight(app: &mut App, bodyweight_id: u64) -> Result<(), AppInputError> {
    match app.service.delete_bodyweight(bodyweight_id as i64) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error deleting bodyweight: {}",
            e
        ))),
    }
}

pub fn handle_create_exercise_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;
    let mut focus_changed = false; // To potentially trigger re-renders if needed

    if let ActiveModal::CreateExercise {
        ref mut name_input,
        ref mut muscles_input,
        ref mut selected_type,
        ref mut focused_field,
        ref mut error_message,
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
                AddExerciseField::Confirm => *focused_field = AddExerciseField::TypeBodyweight,
                AddExerciseField::Cancel => *focused_field = AddExerciseField::Confirm,
            }
            focus_changed = true;
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
                        focus_changed = true;
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
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Name;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                // --- Type Selection Fields ---
                AddExerciseField::TypeResistance => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::Resistance, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::TypeCardio;
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        // Wrap around left (or could go to Muscles - Tab/Shift-Tab is better)
                        *focused_field = AddExerciseField::Cancel;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeCardio => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::Cardio, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        *focused_field = AddExerciseField::TypeResistance;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles; // Jump back to Muscles
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddExerciseField::TypeBodyweight => match key.code {
                    KeyCode::Enter => *selected_type = ExerciseType::BodyWeight, // Confirm selection (optional)
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Confirm; // Move to confirm
                        focus_changed = true;
                    }
                    KeyCode::Left => {
                        *focused_field = AddExerciseField::TypeCardio;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddExerciseField::Muscles; // Jump back to Muscles
                        focus_changed = true;
                    }
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
                    KeyCode::Left | KeyCode::Backspace => {
                        *focused_field = AddExerciseField::Cancel;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the types section
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Right | KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Cancel; // Cycle behavior
                        focus_changed = true;
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
                    KeyCode::Right => {
                        *focused_field = AddExerciseField::Confirm;
                        focus_changed = true;
                    }
                    KeyCode::Left | KeyCode::Backspace => {
                        *focused_field = AddExerciseField::Confirm; // Cycle behavior
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        // Jump back up to the types section
                        *focused_field = AddExerciseField::TypeBodyweight;
                        focus_changed = true;
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        *focused_field = AddExerciseField::Name; // Wrap around to top
                        focus_changed = true;
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

fn parse_option_to_input<T>(option: Option<T>) -> String
where
    T: std::fmt::Display,
{
    if let Some(s) = option {
        format!("{}", s)
    } else {
        String::new()
    }
}
