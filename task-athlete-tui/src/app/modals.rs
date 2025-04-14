// task-athlete-tui/src/app/modals.rs
use super::state::{
    ActiveModal, AddExerciseField, AddWorkoutField, App, LogBodyweightField, SetTargetWeightField,
};
use super::AppInputError;
use anyhow::{bail, Result};
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::str::FromStr;
use task_athlete_lib::{DbError, ExerciseDefinition, ExerciseType, Units};

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
                AppInputError::InvalidNumber(format!("'{}' is not a valid integer", trimmed))
            })
            .and_then(|opt_val| {
                // Basic validation (can be extended)
                if let Some(val) = opt_val.as_ref() {
                    // Assuming T supports comparison with 0 (like i64)
                    // This requires a bound, maybe add later if T is generic
                    // if *val < 0 { return Err(AppInputError::InvalidNumber("Value cannot be negative".into())) }
                }
                Ok(opt_val)
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
            .map_err(|_| {
                AppInputError::InvalidNumber(format!("'{}' is not a valid number", trimmed))
            })
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
         exercise_input,
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         resolved_exercise, // Use the stored resolved exercise
         .. // ignore focused_field, error_message
     } = modal_state {

        // 1. Validate Exercise Selection
        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
            AppInputError::SelectionRequired // Or a custom error like "Exercise must be selected/validated"
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

        // 4. Bodyweight Logic (Service layer handles this now based on type)
        // We just pass the weight_arg and let the service figure it out.
        // We might need to pass the current bodyweight if the service doesn't have it readily?
        // For now, assume service uses its configured bodyweight if needed.
        let bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight // Pass the configured bodyweight
        } else {
            None
        };


        // 5. Call AppService (pass None for implicit creation args, as exercise is resolved)
        match app.service.add_workout(
            canonical_name,
            app.log_viewed_date, // Use the date currently viewed in the log tab
            sets,
            reps,
            weight_arg, // Pass the weight from the input field
            duration,
            distance_arg, // Pass the distance from the input field
            notes,
            None, // No implicit type needed
            None, // No implicit muscles needed
            bodyweight_to_use,
        ) {
            Ok((_workout_id, pb_info)) => {
                 // TODO: Optionally display PB info in status bar?
                 if let Some(pb) = pb_info {
                    // For now, just print a simple message if any PB achieved
                    if pb.any_pb() {
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
         Err(AppInputError::DbError("Internal error: Invalid modal state".to_string()))
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
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error setting target: {}", // ConfigError usually doesn't need DbError type
            e
        ))),
    }
}

fn submit_clear_target_weight(app: &mut App) -> Result<(), AppInputError> {
    match app.service.set_target_bodyweight(None) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppInputError::DbError(format!(
            "Error clearing target: {}",
            e
        ))),
    }
}

fn submit_create_exercise(app: &mut App, modal_state: &ActiveModal) -> Result<(), AppInputError> {
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
pub fn handle_add_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<(), AppInputError> = Ok(());
    let mut should_submit = false;

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
        ref mut resolved_exercise, // Mutable access to store resolved def
    } = app.active_modal
    {
        // Always clear error on any input
        *error_message = None;
        let mut focus_changed = false; // Track if focus changes due to input

        match focused_field {
            AddWorkoutField::Exercise => match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None; // Invalidate resolution when typing
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None; // Invalidate resolution when typing
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    // Attempt to resolve exercise before moving focus
                    match app.service.resolve_exercise_identifier(exercise_input) {
                        Ok(Some(def)) => {
                            *exercise_input = def.name.clone(); // Update input to canonical name
                            *resolved_exercise = Some(def);
                            *focused_field = AddWorkoutField::Sets;
                            focus_changed = true;
                        }
                        Ok(None) => {
                            *error_message =
                                Some(format!("Exercise '{}' not found.", exercise_input));
                            *resolved_exercise = None;
                        }
                        Err(e) => {
                            *error_message = Some(format!("Error: {}", e));
                            *resolved_exercise = None;
                        }
                    }
                }
                KeyCode::Up => *focused_field = AddWorkoutField::Cancel, // Wrap around
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Sets => match key.code {
                KeyCode::Char(c) if c.is_digit(10) => sets_input.push(c),
                KeyCode::Backspace => {
                    sets_input.pop();
                }
                KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(0i64), false),
                KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(0i64), false),
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Reps;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Reps => match key.code {
                KeyCode::Char(c) if c.is_digit(10) => reps_input.push(c),
                KeyCode::Backspace => {
                    reps_input.pop();
                }
                KeyCode::Up => modify_numeric_input(reps_input, 1i64, Some(0i64), false),
                KeyCode::Down => modify_numeric_input(reps_input, -1i64, Some(0i64), false),
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Weight;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Sets;
                    focus_changed = true;
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Weight => {
                let weight_unit = match app.service.config.units {
                    Units::Metric => "kg",
                    Units::Imperial => "lbs",
                };
                let added_label = resolved_exercise
                    .as_ref()
                    .map_or(false, |def| def.type_ == ExerciseType::BodyWeight);
                // TODO: Update label dynamically in UI to show "(Added Weight)" or similar

                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => weight_input.push(c),
                    KeyCode::Backspace => {
                        weight_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(weight_input, 0.5f64, Some(0.0f64), true), // Adjust step as needed
                    KeyCode::Down => {
                        modify_numeric_input(weight_input, -0.5f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Duration;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Reps;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            AddWorkoutField::Duration => match key.code {
                KeyCode::Char(c) if c.is_digit(10) => duration_input.push(c),
                KeyCode::Backspace => {
                    duration_input.pop();
                }
                KeyCode::Up => modify_numeric_input(duration_input, 1i64, Some(0i64), false), // Increment by 1 min
                KeyCode::Down => modify_numeric_input(duration_input, -1i64, Some(0i64), false),
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Distance;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Weight;
                    focus_changed = true;
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Distance => {
                let dist_unit = match app.service.config.units {
                    Units::Metric => "km",
                    Units::Imperial => "mi",
                };
                // TODO: Update label dynamically in UI

                match key.code {
                    KeyCode::Char(c) if "0123456789.".contains(c) => distance_input.push(c),
                    KeyCode::Backspace => {
                        distance_input.pop();
                    }
                    KeyCode::Up => modify_numeric_input(distance_input, 0.1f64, Some(0.0f64), true), // Increment by 0.1
                    KeyCode::Down => {
                        modify_numeric_input(distance_input, -0.1f64, Some(0.0f64), true)
                    }
                    KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                        *focused_field = AddWorkoutField::Notes;
                        focus_changed = true;
                    }
                    KeyCode::Up => {
                        *focused_field = AddWorkoutField::Duration;
                        focus_changed = true;
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            AddWorkoutField::Notes => match key.code {
                KeyCode::Char(c) => notes_input.push(c),
                KeyCode::Backspace => {
                    notes_input.pop();
                }
                KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Confirm;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Distance;
                    focus_changed = true;
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Confirm => match key.code {
                KeyCode::Enter => {
                    // ONLY set the flag here. Do NOT clone yet.
                    should_submit = true;
                }
                KeyCode::Left | KeyCode::Backspace => {
                    *focused_field = AddWorkoutField::Cancel;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Notes;
                    focus_changed = true;
                }
                KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Cancel;
                    focus_changed = true;
                }
                KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                _ => {}
            },
            AddWorkoutField::Cancel => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    app.active_modal = ActiveModal::None;
                    return Ok(());
                }
                KeyCode::Right => {
                    *focused_field = AddWorkoutField::Confirm;
                    focus_changed = true;
                }
                KeyCode::Up => {
                    *focused_field = AddWorkoutField::Notes;
                    focus_changed = true;
                }
                KeyCode::Down | KeyCode::Tab => {
                    *focused_field = AddWorkoutField::Exercise;
                    focus_changed = true;
                }
                _ => {}
            },
        }

        // If focus moved away from Exercise field, try to resolve if not already resolved
        if focus_changed
            && *focused_field != AddWorkoutField::Exercise
            && resolved_exercise.is_none()
        {
            match app.service.resolve_exercise_identifier(exercise_input) {
                Ok(Some(def)) => {
                    *exercise_input = def.name.clone(); // Update input to canonical name
                    *resolved_exercise = Some(def);
                }
                Ok(None) => {
                    *error_message = Some(format!("Exercise '{}' not found.", exercise_input));
                } // Non-blocking error
                Err(e) => {
                    *error_message = Some(format!("Error: {}", e));
                }
            }
        }
    } // End mutable borrow of app.active_modal

    // --- Submission Logic (runs only if should_submit is true) ---
    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        // Ensure it's still the correct modal type before submitting
        if let ActiveModal::AddWorkout { .. } = modal_state_clone {
            submission_result = submit_add_workout(app, &modal_state_clone); // Pass the clone
        } else {
            // Handle unlikely case where modal changed state somehow
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed unexpectedly".to_string(),
            ));
        }

        // --- Handle Submission Result ---
        if submission_result.is_ok() {
            app.active_modal = ActiveModal::None; // Close modal on success
                                                  // Refresh handled by main loop
        } else {
            // Submission failed, need to put error back into modal state
            // Re-borrow mutably ONLY if necessary to set error
            if let ActiveModal::AddWorkout {
                ref mut error_message,
                ..
            } = app.active_modal
            {
                *error_message = Some(submission_result.unwrap_err().to_string());
                // Keep the modal open
            }
            // If modal somehow changed state between submit check and here, error is lost (unlikely)
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
