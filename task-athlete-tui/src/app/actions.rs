use super::state::{ActiveModal, AddExerciseField, AddWorkoutField, App};
use anyhow::Result;
use ratatui::widgets::ListState;
use task_athlete_lib::{ExerciseDefinition, ExerciseType, Units, Workout};

// Make handle_key_event a method on App
impl App {
    pub fn open_add_workout_modal(&mut self) -> Result<()> {
        let mut initial_exercise_input = String::new();
        let mut initial_sets = "1".to_string();
        let mut initial_reps = String::new();
        let mut initial_weight = String::new();
        let mut initial_duration = String::new();
        let mut initial_distance = String::new();
        let initial_notes = String::new();
        let mut resolved_exercise = None;

        // Fetch all identifiers for suggestions
        let all_identifiers = self.get_all_exercise_identifiers();

        // Try to pre-fill from selected exercise's last entry
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) = self.log_exercises_today.get(selected_index) {
                initial_exercise_input = selected_exercise_name.clone();
                match self
                    .service
                    .resolve_exercise_identifier(selected_exercise_name)
                {
                    Ok(Some(def)) => {
                        let last_workout = self.get_last_or_specific_workout(&def.name, None);
                        self.populate_workout_inputs_from_def_and_last_workout(
                            &def,
                            last_workout,
                            &mut initial_sets,
                            &mut initial_reps,
                            &mut initial_weight,
                            &mut initial_duration,
                            &mut initial_distance,
                        );
                        resolved_exercise = Some(def.clone());
                    }
                    Ok(None) => { /* Handle unlikely case where selected name doesn't resolve */ }
                    Err(e) => {
                        self.set_error(format!("Error resolving exercise: {}", e));
                        // Proceed without pre-filling fields if resolution fails
                    }
                }
            }
        }

        self.active_modal = ActiveModal::AddWorkout {
            exercise_input: initial_exercise_input,
            sets_input: initial_sets,
            reps_input: initial_reps,
            weight_input: initial_weight,
            duration_input: initial_duration,
            distance_input: initial_distance,
            notes_input: initial_notes,
            focused_field: AddWorkoutField::Exercise,
            error_message: None,
            resolved_exercise,
            all_exercise_identifiers: all_identifiers,
            exercise_suggestions: Vec::new(), // Start with empty suggestions ALWAYS
            suggestion_list_state: ListState::default(),
        };
        Ok(())
    }

    // Helper to populate workout fields based on resolved exercise and last workout
    fn populate_workout_inputs_from_def_and_last_workout(
        &self,
        def: &ExerciseDefinition,
        last_workout_opt: Option<Workout>,
        sets_input: &mut String,
        reps_input: &mut String,
        weight_input: &mut String,
        duration_input: &mut String,
        distance_input: &mut String,
        // notes_input: &mut String, // Notes are usually not pre-filled
    ) {
        if let Some(last_workout) = last_workout_opt {
            *sets_input = last_workout.sets.map_or("1".to_string(), |v| v.to_string());
            *reps_input = last_workout.reps.map_or(String::new(), |v| v.to_string());
            *duration_input = last_workout
                .duration_minutes
                .map_or(String::new(), |v| v.to_string());
            // *notes_input = last_workout.notes.clone().unwrap_or_default(); // Optionally prefill notes

            // Weight logic
            if def.type_ == ExerciseType::BodyWeight {
                let bodyweight_used = self.service.config.bodyweight.unwrap_or(0.0);
                let added_weight = last_workout
                    .weight
                    .map_or(0.0, |w| w - bodyweight_used)
                    .max(0.0);
                *weight_input = if added_weight > 0.0 {
                    format!("{:.1}", added_weight)
                } else {
                    String::new() // Clear if only bodyweight was used
                };
            } else {
                *weight_input = last_workout
                    .weight
                    .map_or(String::new(), |v| format!("{:.1}", v));
            }

            // Distance Logic
            if let Some(dist_km) = last_workout.distance {
                let display_dist = match self.service.config.units {
                    Units::Metric => dist_km,
                    Units::Imperial => dist_km * 0.621371,
                };
                *distance_input = format!("{:.1}", display_dist);
            } else {
                *distance_input = String::new(); // Clear distance if not present
            }
        } else {
            // Reset fields if no last workout found for this exercise
            *sets_input = "1".to_string();
            *reps_input = String::new();
            *weight_input = String::new();
            *duration_input = String::new();
            *distance_input = String::new();
            // *notes_input = String::new();
        }
    }

    fn populate_workout_inputs_from_def_and_workout(
        &self,
        def: &ExerciseDefinition,
        workout: &Workout, // The specific workout being edited
        sets_input: &mut String,
        reps_input: &mut String,
        weight_input: &mut String,
        duration_input: &mut String,
        distance_input: &mut String,
        notes_input: &mut String,
    ) {
        *sets_input = workout.sets.map_or("1".to_string(), |v| v.to_string());
        *reps_input = workout.reps.map_or(String::new(), |v| v.to_string());
        *duration_input = workout
            .duration_minutes
            .map_or(String::new(), |v| v.to_string());
        *notes_input = workout.notes.clone().unwrap_or_default();

        // Weight logic (same as before, but applied to the specific workout's weight)
        if def.type_ == ExerciseType::BodyWeight {
            let bodyweight_used = self.service.config.bodyweight.unwrap_or(0.0);
            let added_weight = workout.weight.map_or(0.0, |w| w - bodyweight_used).max(0.0);
            *weight_input = if added_weight > 0.0 {
                format!("{:.1}", added_weight)
            } else {
                String::new()
            };
        } else {
            *weight_input = workout
                .weight
                .map_or(String::new(), |v| format!("{:.1}", v));
        }

        // Distance Logic (same as before)
        if let Some(dist_km) = workout.distance {
            let display_dist = match self.service.config.units {
                Units::Metric => dist_km,
                Units::Imperial => dist_km * 0.621371,
            };
            *distance_input = format!("{:.1}", display_dist);
        } else {
            *distance_input = String::new();
        }
    }

    pub fn open_edit_workout_modal(&mut self) -> Result<()> {
        let selected_set_index = match self.log_set_table_state.selected() {
            Some(i) => i,
            None => return Ok(()), // No set selected, do nothing
        };

        let workout_to_edit = match self.log_sets_for_selected_exercise.get(selected_set_index) {
            Some(w) => w.clone(),  // Clone to avoid borrow issues
            None => return Ok(()), // Index out of bounds (shouldn't happen)
        };

        let mut sets_input = "1".to_string();
        let mut reps_input = String::new();
        let mut weight_input = String::new();
        let mut duration_input = String::new();
        let mut distance_input = String::new();
        let mut notes_input = String::new();
        let mut resolved_exercise = None;

        // Get definition and *this specific workout's* data for fields
        // We pass the workout_id here to potentially load *that* specific workout if needed,
        // but populate_workout_inputs currently uses the *last* workout for hints.
        // We will override with the actual data below.
        match self.get_data_for_workout_modal(
            &workout_to_edit.exercise_name,
            Some(workout_to_edit.id as u64),
        ) {
            Ok((def, _)) => {
                // We don't need the last_workout here, we have the specific one
                // Populate directly from the workout being edited
                self.populate_workout_inputs_from_def_and_workout(
                    &def,
                    &workout_to_edit, // Use the specific workout
                    &mut sets_input,
                    &mut reps_input,
                    &mut weight_input,
                    &mut duration_input,
                    &mut distance_input,
                    &mut notes_input,
                );
                resolved_exercise = Some(def.clone());
            }
            Err(e) => {
                self.set_error(format!("Error getting exercise details: {}", e));
                return Ok(()); // Don't open modal if we can't get details
            }
        }

        self.active_modal = ActiveModal::EditWorkout {
            workout_id: workout_to_edit.id as u64,
            exercise_name: workout_to_edit.exercise_name.clone(), // Store for display
            sets_input,
            reps_input,
            weight_input,
            duration_input,
            distance_input,
            notes_input,
            focused_field: AddWorkoutField::Sets, // Start focus on Sets (exercise not editable)
            error_message: None,
            resolved_exercise,
        };

        Ok(())
    }

    // NEW: Open Delete Confirmation Modal
    pub fn open_delete_confirmation_modal(&mut self) -> Result<()> {
        let selected_index = match self.log_set_table_state.selected() {
            Some(i) => i,
            None => return Ok(()), // No set selected
        };

        if let Some(workout) = self.log_sets_for_selected_exercise.get(selected_index) {
            self.active_modal = ActiveModal::ConfirmDeleteWorkout {
                workout_id: workout.id as u64,
                exercise_name: workout.exercise_name.clone(),
                set_index: selected_index + 1, // Display 1-based index
            };
        }

        Ok(())
    }

    pub fn filter_exercise_suggestions(&mut self) {
        if let ActiveModal::AddWorkout {
             ref exercise_input,
             ref all_exercise_identifiers,
             ref mut exercise_suggestions,
             ref mut suggestion_list_state,
             .. // ignore other fields
         } = self.active_modal {
            let input_lower = exercise_input.to_lowercase();
            if input_lower.is_empty() {
                *exercise_suggestions = Vec::new(); // Clear suggestions if input is empty
            } else {
                *exercise_suggestions = all_exercise_identifiers
                    .iter()
                    .filter(|identifier| identifier.to_lowercase().starts_with(&input_lower))
                    .take(5).cloned() // Limit suggestions to a reasonable number (e.g., 5)
                    .collect();
            }
             // Reset selection when suggestions change
            suggestion_list_state.select(if exercise_suggestions.is_empty() { None } else { Some(0) });
         }
    }

    pub fn open_create_exercise_modal(&mut self) -> Result<()> {
        self.active_modal = ActiveModal::CreateExercise {
            name_input: String::new(),
            muscles_input: String::new(),
            selected_type: ExerciseType::Resistance, // Default to Resistance
            focused_field: AddExerciseField::Name,   // Start focus on name
            log_weight: true,
            log_reps: true,
            error_message: None,
            log_duration: true,
            log_distance: true,
        };
        Ok(())
    }

    // Keep cycle graph range here as it modifies App state directly
    pub fn bw_cycle_graph_range(&mut self) {
        self.bw_graph_range_months = match self.bw_graph_range_months {
            1 => 3,
            3 => 6,
            6 => 12,
            12 => 0,
            _ => 1,
        };
        self.update_bw_graph_data(); // Call data update method
    }

    pub fn open_delete_bodyweight_confirmation_modal(&mut self) -> Result<()> {
        let selected_index = match self.bw_history_state.selected() {
            Some(i) => i,
            None => return Ok(()), // No set selected
        };

        if let Some(bodyweight) = self.bw_history.get(selected_index) {
            self.active_modal = ActiveModal::ConfirmDeleteBodyWeight {
                body_weight_id: bodyweight.0 as u64,
                set_index: selected_index + 1, // Display 1-based index
            };
        }

        Ok(())
    }

    pub fn open_pb_modal(&mut self, exercise_name: String, pb_info: task_athlete_lib::PBInfo) {
        self.active_modal = ActiveModal::PersonalBest {
            exercise_name,
            pb_info,
            focused_field: super::state::PbModalField::OkButton,
        };
    }

    fn get_data_for_workout_modal(
        &mut self,
        exercise_identifier: &str,
        workout_id_for_context: Option<u64>, // Pass Some(id) when editing
    ) -> Result<(ExerciseDefinition, Option<Workout>), anyhow::Error> {
        let def = self
            .service
            .resolve_exercise_identifier(exercise_identifier)?
            .ok_or_else(|| anyhow::anyhow!("Exercise '{}' not found.", exercise_identifier))?;
        let last_workout = self.get_last_or_specific_workout(&def.name, workout_id_for_context);
        Ok((def, last_workout))
    }
}
