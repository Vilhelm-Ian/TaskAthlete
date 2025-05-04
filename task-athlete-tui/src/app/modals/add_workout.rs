// src/app/modals/add_workout.rs
// ... other imports ...
use super::input_helpers::{get_next_focusable_field, NavigationDirection};
use crate::app::state::{ActiveModal, AddWorkoutField, App, WorkoutLogFlags};
use crate::app::utils::parse_option_to_input;
use crate::app::utils::{modify_numeric_input, parse_optional_float, parse_optional_int};
use crate::app::AppInputError;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use task_athlete_lib::{AddWorkoutParams, DbError, ExerciseDefinition, ExerciseType};

// --- Submission Logic --- (no changes needed here)
fn submit_add_workout(app: &mut App, modal_state: &ActiveModal) -> Result<bool, AppInputError> {
    // ... same as before ...
    if let ActiveModal::AddWorkout {
        sets_input,
        reps_input,
        weight_input,
        duration_input,
        distance_input,
        notes_input,
        resolved_exercise,
        ..
    } = modal_state
    {
        let mut workout_parameters = AddWorkoutParams::default();
        workout_parameters.date = if Utc::now().date_naive() == app.log_viewed_date {
            Utc::now()
        } else {
            let naive = app
                .log_viewed_date
                .and_hms_opt(12, 0, 0)
                .ok_or_else(|| AppInputError::InvalidDate("Invalid date".to_string()))?;
            Utc.from_utc_datetime(&naive)
        };

        let exercise_def = resolved_exercise.as_ref().ok_or_else(|| {
            AppInputError::DbError("Exercise not resolved. Select a valid exercise.".to_string())
        })?;
        workout_parameters.exercise_identifier = exercise_def.name.as_str();

        let flags = WorkoutLogFlags::from_def(Some(exercise_def));
        workout_parameters.sets = if flags.log_sets {
            parse_optional_int::<i64>(sets_input)?
        } else {
            None
        };
        workout_parameters.reps = if flags.log_reps {
            parse_optional_int::<i64>(reps_input)?
        } else {
            None
        };
        workout_parameters.weight = if flags.log_weight {
            parse_optional_float(weight_input)?
        } else {
            None
        };
        workout_parameters.duration = if flags.log_duration {
            parse_optional_int::<i64>(duration_input)?
        } else {
            None
        };
        workout_parameters.distance = if flags.log_distance {
            parse_optional_float(distance_input)?
        } else {
            None
        };

        workout_parameters.notes = if notes_input.trim().is_empty() {
            None
        } else {
            Some(notes_input.trim().to_string())
        };

        workout_parameters.bodyweight_to_use = if exercise_def.type_ == ExerciseType::BodyWeight {
            app.service.config.bodyweight
        } else {
            None
        };
        let ex_identifier = workout_parameters.exercise_identifier;

        match app.service.add_workout(workout_parameters) {
            Ok((_workout_id, pb_info)) => {
                let mut pb_modal_opened = false;
                if let Some(pb) = pb_info {
                    if pb.any_pb() {
                        app.open_pb_modal(ex_identifier.to_string(), pb);
                        pb_modal_opened = true;
                    }
                }
                Ok(pb_modal_opened)
            }
            Err(e) => {
                if let Some(db_err) = e.downcast_ref::<DbError>() {
                    Err(AppInputError::DbError(db_err.to_string()))
                } else if let Some(cfg_err) = e.downcast_ref::<task_athlete_lib::ConfigError>() {
                    Err(AppInputError::DbError(cfg_err.to_string()))
                } else {
                    Err(AppInputError::DbError(format!(
                        "Error adding workout: {}",
                        e
                    )))
                }
            }
        }
    } else {
        Err(AppInputError::DbError(
            "Internal error: Invalid modal state for add workout".to_string(),
        ))
    }
}

// --- Input Handling ---

pub fn handle_add_workout_modal_input(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut submission_result: Result<bool, AppInputError> = Ok(false); // Store PB modal flag
    let mut should_submit = false;
    let mut needs_suggestion_update = false;
    let mut repopulate_fields_for_resolved_exercise: Option<ExerciseDefinition> = None;
    let mut next_focus_target: Option<AddWorkoutField> = None; // <-- Store next focus target

    // --- Get Flags and Current Focus Early ---
    let (current_focused_field, flags, is_add_mode) = {
        if let ActiveModal::AddWorkout {
            focused_field,
            resolved_exercise,
            ..
        } = &app.active_modal
        {
            (
                *focused_field,
                WorkoutLogFlags::from_def(resolved_exercise.as_ref()),
                true,
            )
        } else {
            (AddWorkoutField::Cancel, WorkoutLogFlags::default(), true)
        }
    };

    // --- Main Input Handling Logic (Inside mutable borrow) ---
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
        ..
    } = app.active_modal
    {
        *error_message = None;

        // Handle Suggestions state separately
        if *focused_field == AddWorkoutField::Suggestions {
            match key.code {
                KeyCode::Char(c) => {
                    exercise_input.push(c);
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    next_focus_target = Some(AddWorkoutField::Exercise); // Go back to input field
                }
                KeyCode::Backspace => {
                    exercise_input.pop();
                    *resolved_exercise = None;
                    needs_suggestion_update = true;
                    next_focus_target = Some(AddWorkoutField::Exercise); // Go back to input field
                }
                KeyCode::Up => {
                    /* ... suggestion list navigation ... */
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
                    /* ... suggestion list navigation ... */
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
                        if let Some(selected_suggestion) = exercise_suggestions.get(selected_index)
                        {
                            let suggestion_clone = selected_suggestion.clone();
                            match app.service.resolve_exercise_identifier(&suggestion_clone) {
                                Ok(Some(def)) => {
                                    *exercise_input = def.name.clone();
                                    if resolved_exercise.as_ref() != Some(&def) {
                                        repopulate_fields_for_resolved_exercise = Some(def.clone());
                                    }
                                    *resolved_exercise = Some(def);
                                    next_focus_target = Some(get_next_focusable_field(
                                        current_focused_field,
                                        &flags,
                                        NavigationDirection::Forward,
                                        is_add_mode,
                                    ));
                                    exercise_suggestions.clear();
                                    suggestion_list_state.select(None);
                                }
                                Ok(None) => {
                                    *error_message = Some(format!(
                                        "Could not resolve selected '{}'.",
                                        suggestion_clone
                                    ));
                                    next_focus_target = Some(AddWorkoutField::Exercise);
                                }
                                Err(e) => {
                                    *error_message =
                                        Some(format!("Error resolving selected: {}", e));
                                    next_focus_target = Some(AddWorkoutField::Exercise);
                                }
                            }
                        }
                    } else {
                        // Try resolving current input if Enter hit with no selection
                        let input_clone = exercise_input.clone();
                        match app.service.resolve_exercise_identifier(&input_clone) {
                            Ok(Some(def)) => {
                                *exercise_input = def.name.clone();
                                if resolved_exercise.as_ref() != Some(&def) {
                                    repopulate_fields_for_resolved_exercise = Some(def.clone());
                                }
                                *resolved_exercise = Some(def);
                                next_focus_target = Some(get_next_focusable_field(
                                    current_focused_field,
                                    &flags,
                                    NavigationDirection::Forward,
                                    is_add_mode,
                                ));
                                exercise_suggestions.clear();
                                suggestion_list_state.select(None);
                            }
                            Ok(None) => {
                                next_focus_target = Some(AddWorkoutField::Exercise);
                            }
                            Err(e) => {
                                *error_message = Some(format!("Error resolving input: {}", e));
                                next_focus_target = Some(AddWorkoutField::Exercise);
                            }
                        }
                    }
                }
                KeyCode::Tab | KeyCode::Esc => {
                    next_focus_target = Some(AddWorkoutField::Exercise);
                }
                _ => {}
            }
        } else {
            // Handle all other fields
            match current_focused_field {
                // Use the immutable `current_focused_field`
                AddWorkoutField::Exercise => match key.code {
                    KeyCode::Char(c) => {
                        exercise_input.push(c);
                        *resolved_exercise = None;
                        needs_suggestion_update = true;
                    }
                    KeyCode::Backspace => {
                        exercise_input.pop();
                        *resolved_exercise = None;
                        needs_suggestion_update = true;
                    }
                    KeyCode::Down => {
                        if !exercise_suggestions.is_empty() {
                            next_focus_target = Some(AddWorkoutField::Suggestions); // Request move to suggestions
                            suggestion_list_state.select(Some(0));
                        } else {
                            let input_clone = exercise_input.clone();
                            match app.service.resolve_exercise_identifier(&input_clone) {
                                Ok(Some(def)) => {
                                    *exercise_input = def.name.clone();
                                    if resolved_exercise.as_ref() != Some(&def) {
                                        repopulate_fields_for_resolved_exercise = Some(def.clone());
                                    }
                                    *resolved_exercise = Some(def);
                                    next_focus_target = Some(get_next_focusable_field(
                                        current_focused_field,
                                        &flags,
                                        NavigationDirection::Forward,
                                        is_add_mode,
                                    ));
                                    exercise_suggestions.clear();
                                    suggestion_list_state.select(None);
                                }
                                Ok(None) => {
                                    *error_message =
                                        Some(format!("Exercise '{}' not found.", input_clone));
                                }
                                Err(e) => *error_message = Some(format!("Error: {}", e)),
                            }
                        }
                    }
                    KeyCode::Tab => {
                        let input_clone = exercise_input.clone();
                        if resolved_exercise.is_none() && !input_clone.is_empty() {
                            match app.service.resolve_exercise_identifier(&input_clone) {
                                Ok(Some(def)) => {
                                    *exercise_input = def.name.clone();
                                    if resolved_exercise.as_ref() != Some(&def) {
                                        repopulate_fields_for_resolved_exercise = Some(def.clone());
                                    }
                                    *resolved_exercise = Some(def);
                                    next_focus_target = Some(get_next_focusable_field(
                                        current_focused_field,
                                        &flags,
                                        NavigationDirection::Forward,
                                        is_add_mode,
                                    ));
                                    exercise_suggestions.clear();
                                    suggestion_list_state.select(None);
                                }
                                Ok(None) => {
                                    *error_message = Some(format!(
                                        "Exercise '{}' not found. Cannot move.",
                                        input_clone
                                    ));
                                }
                                Err(e) => {
                                    *error_message =
                                        Some(format!("Error resolving: {}. Cannot move.", e));
                                }
                            }
                        } else {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ));
                            exercise_suggestions.clear();
                            suggestion_list_state.select(None);
                        }
                    }
                    KeyCode::Up => {
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Backward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Sets => {
                    exercise_suggestions.clear();
                    suggestion_list_state.select(None);
                    match key.code {
                        KeyCode::Char(c) if c.is_ascii_digit() => sets_input.push(c),
                        KeyCode::Backspace => {
                            sets_input.pop();
                        }
                        KeyCode::Up => modify_numeric_input(sets_input, 1i64, Some(1i64), false),
                        KeyCode::Down => modify_numeric_input(sets_input, -1i64, Some(1i64), false),
                        KeyCode::Enter | KeyCode::Tab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::BackTab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
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
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Forward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::BackTab => {
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Backward,
                            is_add_mode,
                        ))
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
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Forward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::BackTab => {
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Backward,
                            is_add_mode,
                        ))
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
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Forward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::BackTab => {
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Backward,
                            is_add_mode,
                        ))
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
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Forward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::BackTab => {
                        next_focus_target = Some(get_next_focusable_field(
                            current_focused_field,
                            &flags,
                            NavigationDirection::Backward,
                            is_add_mode,
                        ))
                    }
                    KeyCode::Esc => {
                        app.active_modal = ActiveModal::None;
                        return Ok(());
                    }
                    _ => {}
                },
                AddWorkoutField::Notes => {
                    exercise_suggestions.clear();
                    suggestion_list_state.select(None);
                    match key.code {
                        KeyCode::Char(c) => notes_input.push(c),
                        KeyCode::Backspace => {
                            notes_input.pop();
                        }
                        KeyCode::Enter | KeyCode::Tab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::BackTab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Confirm => {
                    exercise_suggestions.clear();
                    suggestion_list_state.select(None);
                    match key.code {
                        KeyCode::Enter => {
                            if resolved_exercise.is_none() {
                                *error_message =
                                    Some("Cannot submit: Exercise not resolved.".to_string());
                                next_focus_target = Some(AddWorkoutField::Exercise);
                            } else {
                                should_submit = true;
                            }
                        }
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Up => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Down | KeyCode::Tab | KeyCode::Right => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                AddWorkoutField::Cancel => {
                    exercise_suggestions.clear();
                    suggestion_list_state.select(None);
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            app.active_modal = ActiveModal::None;
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Tab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Left | KeyCode::Backspace | KeyCode::BackTab => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Up => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Backward,
                                is_add_mode,
                            ))
                        }
                        KeyCode::Down => {
                            next_focus_target = Some(get_next_focusable_field(
                                current_focused_field,
                                &flags,
                                NavigationDirection::Forward,
                                is_add_mode,
                            ))
                        }
                        _ => {}
                    }
                }
                // Suggestions state is handled above
                AddWorkoutField::Suggestions => {}
            }
        }

        // --- Fallback Resolution ---
        if let Some(target) = next_focus_target {
            if target != AddWorkoutField::Exercise
                && target != AddWorkoutField::Suggestions
                && resolved_exercise.is_none()
                && !exercise_input.is_empty()
                && is_add_mode
            {
                let input_clone = exercise_input.clone();
                match app.service.resolve_exercise_identifier(&input_clone) {
                    Ok(Some(def)) => {
                        *exercise_input = def.name.clone();
                        if resolved_exercise.as_ref() != Some(&def) {
                            repopulate_fields_for_resolved_exercise = Some(def.clone());
                        }
                        *resolved_exercise = Some(def);
                    }
                    Ok(None) => {
                        *resolved_exercise = None;
                    } // Clear resolution if failed
                    Err(e) => {
                        *resolved_exercise = None;
                        *error_message = Some(format!("Error resolving '{}': {}", input_clone, e));
                    }
                }
            }
        }

        // --- Apply Focus Change AFTER the match ---
        if let Some(target) = next_focus_target {
            *focused_field = target;
        }
    } // End mutable borrow of app.active_modal

    // --- Repopulate / Suggestions / Submit (outside borrow) ---
    // ... (keep existing logic here) ...
    if let Some(def_to_repopulate) = repopulate_fields_for_resolved_exercise {
        let last_workout = app.get_last_or_specific_workout(&def_to_repopulate.name, None);
        if let ActiveModal::AddWorkout {
            ref mut sets_input,
            ref mut reps_input,
            ref mut weight_input,
            ref mut duration_input,
            ref mut distance_input,
            ..
        } = app.active_modal
        {
            let flags = WorkoutLogFlags::from_def(Some(&def_to_repopulate)); // Get flags for repopulation

            if flags.log_sets {
                *sets_input = last_workout
                    .as_ref()
                    .map_or_else(|| "1".to_string(), |w| parse_option_to_input(w.sets));
            } else {
                *sets_input = String::new();
            }
            if flags.log_reps {
                *reps_input = last_workout
                    .as_ref()
                    .map_or_else(String::new, |w| parse_option_to_input(w.reps));
            } else {
                *reps_input = String::new();
            }
            if flags.log_weight {
                *weight_input = last_workout
                    .as_ref()
                    .map_or_else(String::new, |w| parse_option_to_input(w.weight));
            } else {
                *weight_input = String::new();
            }
            if flags.log_duration {
                *duration_input = last_workout
                    .as_ref()
                    .map_or_else(String::new, |w| parse_option_to_input(w.duration_minutes));
            } else {
                *duration_input = String::new();
            }
            if flags.log_distance {
                *distance_input = last_workout
                    .as_ref()
                    .map_or_else(String::new, |w| parse_option_to_input(w.distance));
            } else {
                *distance_input = String::new();
            }
        }
    }

    if needs_suggestion_update {
        app.filter_exercise_suggestions();
    }

    if should_submit {
        let modal_state_clone = app.active_modal.clone();
        if let ActiveModal::AddWorkout { .. } = modal_state_clone {
            submission_result = submit_add_workout(app, &modal_state_clone);
        } else {
            submission_result = Err(AppInputError::DbError(
                "Internal Error: Modal state changed".to_string(),
            ));
        }

        match submission_result {
            Ok(pb_modal_opened) => {
                if !pb_modal_opened {
                    app.active_modal = ActiveModal::None;
                }
            }
            Err(err) => {
                if let ActiveModal::AddWorkout {
                    ref mut error_message,
                    ..
                } = app.active_modal
                {
                    *error_message = Some(err.to_string());
                }
            }
        }
    }

    Ok(())
}
