use super::state::App;
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use task_athlete_lib::{DbError, Workout, WorkoutFilters};

// Make refresh logic methods on App
impl App {
    // Fetch or update data based on the active tab
    pub fn refresh_data_for_active_tab(&mut self) {
        self.clear_expired_error(); // Check and clear status bar error first

        match self.active_tab {
            super::state::ActiveTab::Log => self.refresh_log_data(),
            super::state::ActiveTab::History => {} // TODO
            super::state::ActiveTab::Graphs => {}  // TODO
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
        match self.service.list_workouts(filters) {
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

                self.bw_latest = self.bw_history.first().map(|(_, w)| *w);
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
                .map(|(ts, _)| ts.date_naive())
                .unwrap_or(now_naive)
        };

        let filtered_data: Vec<_> = self
            .bw_history
            .iter()
            .filter(|(ts, _)| ts.date_naive() >= start_date_filter)
            .rev()
            .collect();

        if filtered_data.is_empty() {
            self.bw_graph_data.clear();
            return;
        }
        let first_day_epoch = filtered_data
            .first()
            .unwrap()
            .0
            .date_naive()
            .num_days_from_ce();
        self.bw_graph_data = filtered_data
            .iter()
            .map(|(date, weight)| {
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
