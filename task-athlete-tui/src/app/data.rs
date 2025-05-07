use super::state::App;
use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Utc};
// Import Line, Span
use std::collections::HashMap;
use task_athlete_lib::Units;
use task_athlete_lib::{DbError, GraphType as LibGraphType, Workout, WorkoutFilters};

// Make refresh logic methods on App
impl App {
    // Fetch or update data based on the active tab
    pub fn refresh_data_for_active_tab(&mut self) {
        self.clear_expired_error(); // Check and clear status bar error first

        match self.active_tab {
            super::state::ActiveTab::Log => self.refresh_log_data(),
            super::state::ActiveTab::History => self.refresh_history_data(),
            super::state::ActiveTab::Graphs => {} // TODO
            super::state::ActiveTab::Bodyweight => self.refresh_bodyweight_data(),
        }
    }

    // --- Log Tab Data ---
    pub(crate) fn refresh_log_data(&mut self) {
        // Make crate-public if needed by other app modules
        let filters = WorkoutFilters {
            date: Some(self.log_viewed_date),
            ..Default::default()
        };
        match self.service.list_workouts(&filters) {
            Ok(workouts) => {
                let mut unique_names = workouts
                    .iter()
                    .map(|w| w.exercise_name.clone())
                    .collect::<Vec<_>>();
                unique_names.sort_unstable();
                unique_names.dedup();
                self.log_exercises_today = unique_names;

                if self.log_exercise_list_state.selected().unwrap_or(0)
                    >= self.log_exercises_today.len()
                {
                    self.log_exercise_list_state
                        .select(if self.log_exercises_today.is_empty() {
                            None
                        } else {
                            Some(self.log_exercises_today.len().saturating_sub(1))
                        });
                }

                self.update_log_sets_for_selected_exercise(&workouts);
            }
            Err(e) => {
                if e.downcast_ref::<DbError>()
                    .map_or(false, |dbe| matches!(dbe, DbError::ExerciseNotFound(_)))
                {
                    self.log_exercises_today.clear();
                    self.log_sets_for_selected_exercise.clear();
                } else {
                    self.set_error(format!("Error fetching log data: {}", e))
                }
            }
        }
    }

    // Make crate-public
    pub(crate) fn update_log_sets_for_selected_exercise(
        &mut self,
        all_workouts_for_date: &[Workout],
    ) {
        if let Some(selected_index) = self.log_exercise_list_state.selected() {
            if let Some(selected_exercise_name) =
                self.log_exercises_today.get(selected_index).cloned()
            {
                self.log_sets_for_selected_exercise = all_workouts_for_date
                    .iter()
                    .filter(|w| w.exercise_name == selected_exercise_name)
                    .cloned()
                    .collect();

                if self.log_set_table_state.selected().unwrap_or(0)
                    >= self.log_sets_for_selected_exercise.len()
                {
                    self.log_set_table_state.select(
                        if self.log_sets_for_selected_exercise.is_empty() {
                            None
                        } else {
                            Some(self.log_sets_for_selected_exercise.len() - 1)
                        },
                    );
                } else if self.log_set_table_state.selected().is_none()
                    && !self.log_sets_for_selected_exercise.is_empty()
                {
                    self.log_set_table_state.select(Some(0));
                }
            } else {
                self.log_sets_for_selected_exercise.clear();
                self.log_set_table_state.select(None);
            }
        } else {
            self.log_sets_for_selected_exercise.clear();
            self.log_set_table_state.select(None);
        }
    }

    // --- Bodyweight Tab Data ---
    pub(crate) fn refresh_bodyweight_data(&mut self) {
        match self.service.list_bodyweights(1000) {
            Ok(entries) => {
                self.bw_history = entries;

                if self.bw_history_state.selected().unwrap_or(0) >= self.bw_history.len() {
                    self.bw_history_state.select(if self.bw_history.is_empty() {
                        None
                    } else {
                        Some(self.bw_history.len() - 1)
                    });
                } else if self.bw_history_state.selected().is_none() && !self.bw_history.is_empty()
                {
                    self.bw_history_state.select(Some(0));
                }

                self.bw_latest = self.bw_history.first().map(|(_, _, w)| *w);
                self.bw_target = self.service.get_target_bodyweight(); // Refresh target

                self.update_bw_graph_data();
            }
            Err(e) => self.set_error(format!("Error fetching bodyweight data: {}", e)),
        }
    }

    // Make crate-public
    pub(crate) fn update_bw_graph_data(&mut self) {
        if self.bw_history.is_empty() {
            self.bw_graph_data.clear();
            self.bw_graph_x_bounds = [0.0, 1.0];
            self.bw_graph_y_bounds = [0.0, 1.0];
            return;
        }

        let now_naive = Utc::now().date_naive();
        let start_date_filter = if self.bw_graph_range_months > 0 {
            let mut year = now_naive.year();
            let mut month = now_naive.month();
            let day = now_naive.day();
            let months_ago = self.bw_graph_range_months;
            let total_months = (year * 12 + month as i32 - 1) - months_ago as i32;
            year = total_months / 12;
            month = (total_months % 12 + 1) as u32;
            let last_day_of_target_month = NaiveDate::from_ymd_opt(year, month + 1, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap();
            NaiveDate::from_ymd_opt(year, month, day.min(last_day_of_target_month.day()))
                .unwrap_or(last_day_of_target_month)
        } else {
            self.bw_history
                .last()
                .map(|(_, ts, _)| ts.date_naive())
                .unwrap_or(now_naive)
        };

        let filtered_data: Vec<_> = self
            .bw_history
            .iter()
            .filter(|(_, ts, _)| ts.date_naive() >= start_date_filter)
            .rev()
            .collect();

        if filtered_data.is_empty() {
            self.bw_graph_data.clear();
            return;
        }
        let first_day_epoch = filtered_data
            .first()
            .unwrap()
            .1
            .date_naive()
            .num_days_from_ce();
        self.bw_graph_data = filtered_data
            .iter()
            .map(|(_, date, weight)| {
                let days_since_first = (date.num_days_from_ce() - first_day_epoch) as f64;
                (days_since_first, *weight)
            })
            .collect();

        let first_ts = self.bw_graph_data.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last_ts = self
            .bw_graph_data
            .last()
            .map(|(x, _)| *x)
            .unwrap_or(first_ts + 1.0);
        self.bw_graph_x_bounds = [first_ts, last_ts];

        let min_weight = self
            .bw_graph_data
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_weight = self
            .bw_graph_data
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);
        let y_min = self.bw_target.map_or(min_weight, |t| t.min(min_weight));
        let y_max = self.bw_target.map_or(max_weight, |t| t.max(max_weight));
        let y_padding = ((y_max - y_min) * 0.1).max(1.0);
        self.bw_graph_y_bounds = [(y_min - y_padding).max(0.0), y_max + y_padding];
    }
    fn refresh_graphs_tab_data(&mut self) {
        // Load all exercise names if not already loaded
        if self.graph_exercises_all.is_empty() {
            match self.service.list_exercises(None, None) {
                Ok(names) => {
                    self.graph_exercises_all = names.iter().map(|e| e.name.clone()).collect();
                    // Ensure selection doesn't go out of bounds if list was empty
                    if self.graph_exercise_list_state.selected().is_none()
                        && !self.graph_exercises_all.is_empty()
                    {
                        self.graph_exercise_list_state.select(Some(0));
                    }
                }
                Err(e) => self.set_error(format!("Error loading exercise list: {}", e)),
            }
        }

        // Update graph data *only if* an exercise and type are selected
        if self.graph_selected_exercise.is_some() && self.graph_selected_type.is_some() {
            self.update_graph_data();
        } else {
            // Clear graph if no selection
            self.clear_graph_data();
        }
    }

    // Updates the graph data based on current selections
    pub(crate) fn update_graph_data(&mut self) {
        let mut should_clear = false;
        if let (Some(ex_name), Some(graph_type)) = (
            self.graph_selected_exercise.as_ref(), // Take as ref if ex_name is &str
            self.graph_selected_type,
        ) {
            // Pass the date filters to get_data_for_graph
            match self.service.get_data_for_graph(
                ex_name, // Assuming ex_name is &str or can be dereferenced
                graph_type,
                self.graph_start_date_filter, // Use existing or add these fields
                self.graph_end_date_filter,   // to your TUI state struct
            ) {
                Ok(raw_data) if !raw_data.is_empty() => {
                    // raw_data is Vec<(NaiveDate, f64)>

                    // Determine the actual first and last dates from the dataset
                    // unwrap() is safe here because raw_data is not empty.
                    let actual_first_date = raw_data.first().unwrap().0;
                    let actual_last_date = raw_data.last().unwrap().0;

                    // (Optional) Store the first date if you need to map relative f64 days back to NaiveDate
                    // for display purposes (e.g., axis labels, tooltips).
                    // self.graph_actual_first_date = Some(actual_first_date);

                    // Transform Vec<(NaiveDate, f64)> to Vec<(f64, f64)> for plotting
                    // where the f64 x-value is days relative to the actual_first_date.
                    let plot_points: Vec<(f64, f64)> = raw_data
                        .iter()
                        .map(|(date, value)| {
                            let relative_day = (*date - actual_first_date).num_days() as f64;
                            (relative_day, *value)
                        })
                        .collect();

                    self.graph_data_points = plot_points;

                    // Calculate X bounds based on relative days
                    let first_day_f64 = 0.0; // The first data point is now at x = 0.0
                    let mut last_day_f64 = (actual_last_date - actual_first_date).num_days() as f64;

                    // Ensure graph_x_bounds have some width, especially if all data is on the same relative day (0.0)
                    if last_day_f64 < first_day_f64 + 0.1 {
                        // If last_day_f64 is effectively 0.0
                        last_day_f64 = first_day_f64 + 1.0; // Create a window of at least 1.0 unit
                    }
                    self.graph_x_bounds = [first_day_f64, last_day_f64];

                    // Calculate Y bounds (this logic remains largely the same)
                    let min_y = self
                        .graph_data_points
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::INFINITY, f64::min);
                    let max_y = self
                        .graph_data_points
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::NEG_INFINITY, f64::max);

                    // Handle cases like single data point or all y-values being the same for y_padding
                    let y_range = max_y - min_y;
                    let y_padding = if y_range.abs() < f64::EPSILON {
                        // If min_y is very close to max_y
                        (max_y.abs() * 0.1).max(1.0) // 10% of the value, or at least 1.0
                    } else {
                        (y_range * 0.1).max(1.0) // 10% of range, or at least 1.0
                    };

                    self.graph_y_bounds = [(min_y - y_padding).max(0.0), max_y + y_padding];
                }
                Ok(_) => {
                    // Empty data returned (raw_data was empty)
                    should_clear = true;
                    self.set_error(format!(
                        "No data found for '{}' - {}",
                        ex_name,
                        graph_type_to_string(graph_type) // Ensure this helper is available
                    ));
                }
                Err(e) => {
                    should_clear = true;
                    self.set_error(format!("Error loading graph data: {}", e));
                }
            }
        } else {
            should_clear = true;
        }

        if should_clear {
            self.clear_graph_data();
            // Optional: also clear actual_first_date if you added it
            // self.graph_actual_first_date = None;
        }
    }

    // Helper to clear graph state
    fn clear_graph_data(&mut self) {
        self.graph_data_points.clear();
        self.graph_x_bounds = [0.0, 1.0];
        self.graph_y_bounds = [0.0, 1.0];
    }
    // --- History Tab Data ---
    fn refresh_history_data(&mut self) {
        // Fetch *all* workouts (might be inefficient for very large histories)
        let filters = WorkoutFilters {
            ..Default::default()
        };
        match self.service.list_workouts(&filters) {
            Ok(all_workouts) => {
                if all_workouts.is_empty() {
                    self.history_data.clear();
                    self.history_list_state.select(None); // Ensure selection is None if empty
                    return;
                }

                // Group workouts by date
                let mut grouped: HashMap<NaiveDate, Vec<Workout>> = HashMap::new();
                for workout in all_workouts {
                    grouped
                        .entry(workout.timestamp.date_naive())
                        .or_default()
                        .push(workout);
                }

                // Sort workouts within each day (optional, depends on service order)
                for workouts in grouped.values_mut() {
                    workouts.sort_by_key(|w| w.timestamp); // Or by id
                }

                // Convert to Vec and sort by date descending
                let mut sorted_history: Vec<(NaiveDate, Vec<Workout>)> =
                    grouped.into_iter().collect();
                sorted_history.sort_unstable_by_key(|(date, _)| *date);
                sorted_history.reverse(); // Show most recent first

                let old_data_len = self.history_data.len();
                self.history_data = sorted_history;
                let new_data_len = self.history_data.len();

                // Ensure selection is valid after data update
                // If lengths are same, keep selection. If different, select first valid.
                if old_data_len != new_data_len {
                    super::navigation_helpers::ensure_selection_is_valid(
                        &mut self.history_list_state,
                        new_data_len,
                    );
                } else if self.history_list_state.selected().unwrap_or(0) >= new_data_len
                    && new_data_len > 0
                {
                    self.history_list_state.select(Some(new_data_len - 1));
                } else if self.history_list_state.selected().is_none() && new_data_len > 0 {
                    self.history_list_state.select(Some(0));
                }
            }
            Err(e) => {
                self.set_error(format!("Error fetching history data: {}", e));
                self.history_data.clear(); // Clear data on error
                self.history_list_state.select(None);
            }
        }
    }
}

// Function needs to be associated with App or take &mut App
// Move it outside the impl block but keep it in this file, taking &mut App
pub fn log_change_date(app: &mut App, days: i64) {
    if let Some(new_date) = app.log_viewed_date.checked_add_signed(Duration::days(days)) {
        app.log_viewed_date = new_date;
        app.log_exercise_list_state
            .select(if app.log_exercises_today.is_empty() {
                None
            } else {
                Some(0)
            });
        app.log_set_table_state
            .select(if app.log_sets_for_selected_exercise.is_empty() {
                None
            } else {
                Some(0)
            });
        // Data will be refreshed by the main loop
    }
}

pub fn log_set_previous_exercised_date(app: &mut App) -> Result<()> {
    let exercised_dates = app.service.get_all_dates_with_exercise()?;
    let current_date = app.log_viewed_date;
    for date in exercised_dates.into_iter().rev() {
        if date < current_date {
            app.log_viewed_date = date;
            app.log_exercise_list_state
                .select(if app.log_exercises_today.is_empty() {
                    None
                } else {
                    Some(0)
                });
            app.log_set_table_state
                .select(if app.log_sets_for_selected_exercise.is_empty() {
                    None
                } else {
                    Some(0)
                });
            break;
        }
    }
    Ok(())
}

pub fn graph_type_to_string(graph_type: LibGraphType) -> String {
    match graph_type {
        LibGraphType::Estimated1RM => "Estimated 1RM".to_string(),
        LibGraphType::MaxWeight => "Max Weight Lifted".to_string(),
        LibGraphType::MaxReps => "Max Reps Per Set".to_string(),
        LibGraphType::WorkoutVolume => "Workout Volume".to_string(),
        LibGraphType::WorkoutReps => "Total Reps Per Workout".to_string(), // Clarified name
        LibGraphType::WorkoutDuration => "Workout Duration (min)".to_string(),
        LibGraphType::WorkoutDistance => "Workout Distance".to_string(),
    }
}

pub fn log_set_next_exercised_date(app: &mut App) -> Result<()> {
    let exercised_dates = app.service.get_all_dates_with_exercise()?;
    let current_date = app.log_viewed_date;
    for date in exercised_dates {
        if date > current_date {
            app.log_viewed_date = date;
            app.log_exercise_list_state
                .select(if app.log_exercises_today.is_empty() {
                    None
                } else {
                    Some(0)
                });
            app.log_set_table_state
                .select(if app.log_sets_for_selected_exercise.is_empty() {
                    None
                } else {
                    Some(0)
                });
            break;
        }
    }
    Ok(())
}

pub fn format_date_with_ordinal(date: NaiveDate) -> String {
    let day = date.day();
    let suffix = match day {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    };
    // Format like "Saturday 12th April 2025"
    date.format(&format!("%A %-d{} %B %Y", suffix)).to_string()
}

/// Formats a single workout set line for the history view
pub fn format_set_line(workout: &Workout, units: Units) -> String {
    let mut parts = Vec::new();
    if let Some(reps) = workout.reps {
        parts.push(format!("{} reps", reps));
    }
    if let Some(weight_kg) = workout.calculate_effective_weight() {
        let (display_weight, unit_str) = match units {
            Units::Metric => (weight_kg, "kg"),
            Units::Imperial => (weight_kg * 2.20462, "lbs"),
        };
        parts.push(format!("{:.1} {}", display_weight, unit_str));
    }
    if let Some(duration) = workout.duration_minutes {
        parts.push(format!("{} min", duration));
    }
    if let Some(dist_km) = workout.distance {
        let (display_dist, unit_str) = match units {
            Units::Metric => (dist_km, "km"),
            Units::Imperial => (dist_km * 0.621_371, "mi"),
        };
        parts.push(format!("{:.1} {}", display_dist, unit_str));
    }
    if let Some(notes) = &workout.notes {
        if !notes.trim().is_empty() {
            parts.push(format!("({})", notes.trim()));
        }
    }

    parts.join(" x ") // Join parts with " x " or choose another separator
}
