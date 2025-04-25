// src/app/modals/add_workout.rs

use crate::app::state::{ActiveModal, AddWorkoutField, App};
use crate::app::utils::parse_option_to_input;
use crate::app::utils::{modify_numeric_input, parse_optional_float, parse_optional_int}; // Import from sibling utils module
use crate::app::AppInputError;
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use task_athlete_lib::{AddWorkoutParams, DbError, ExerciseDefinition, ExerciseType};

// --- Submission Logic ---

fn submit_add_workout(app: &mut App, modal_state: &ActiveModal) -> Result<bool, AppInputError> {
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
        let mut workout_parameters = AddWorkoutParams::default();
        workout_parameters.date = if Utc::now().date_naive() == app.log_viewed_date {
            Utc::now()
        } else {
            let naive = app.log_viewed_date.and_hms_opt(12, 0, 0)
                .ok_or_else(|| AppInputError::InvalidDate("Invalid date".to_string()))?;
            Utc.from_utc_datetime(&naive)
        };

        // 1. Validate Exercise Selection
        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
             // This error should ideally be prevented by the input handler (not allowing Tab/Enter without resolution)
             AppInputError::DbError("Exercise not resolved. Select a valid exercise.".to_string())
        })?;
        workout_parameters.exercise_identifier = exercise_def.name.as_str();


        // 2. Parse numeric inputs
        workout_parameters.sets = parse_optional_int::<i64>(sets_input)?;

        workout_parameters.reps = parse_optional_int::<i64>(reps_input)?;
        workout_parameters.weight = parse_optional_float(weight_input)?; // This is the value from the input field
        workout_parameters.duration = parse_optional_int::<i64>(duration_input)?;
        workout_parameters.distance = parse_optional_float(distance_input)?; // Value from input field

        // 3. Notes
        workout_parameters.notes = if notes_input.trim().is_empty() { None } else { Some(notes_input.trim().to_string()) };

        // 4. Bodyweight & Units (Service layer handles this based on type and config)
        workout_parameters.bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight // Pass the configured bodyweight
        } else {
            None
        };
        let ex_identifier = workout_parameters.exercise_identifier;
        // 5. Call AppService
        match app.service.add_workout(
            workout_parameters
        ) {
            Ok((_workout_id, pb_info)) => {
                 let mut pb_modal_opened = false; // Initialize the flag
                 if let Some(pb) = pb_info {
                    if pb.any_pb() {
                        app.open_pb_modal(ex_identifier.to_string(), pb);
                        pb_modal_opened = true; // Set the flag if PB modal was opened
                    }
                 }
                 Ok(pb_modal_opened) // Return the flag indicating if PB modal was shown
            }
            Err(e) => {
                dbg!("hello");
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

// --- Input Handling ---

// Made public for re-export in mod.rs
pub fn handle_add_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<bool, AppInputError> = Ok(false);
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
        if let Ok(result) = submission_result {
            if !result {
                app.active_modal = ActiveModal::None; // Close modal on success
                                                      // Data refresh will happen in the main loop
            }
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
