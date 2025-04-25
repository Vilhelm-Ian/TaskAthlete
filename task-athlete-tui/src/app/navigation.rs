use super::{navigation_helpers, state::App};
use task_athlete_lib::WorkoutFilters; // Keep lib imports

// --- Log Tab Navigation ---

// Need to take &mut App now
pub fn log_list_next(app: &mut App) {
    let current_selection = app.log_exercise_list_state.selected();
    let list_len = app.log_exercises_today.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.log_exercise_list_state.select(Some(i));
    // Refresh sets based on new selection (needs access to service or pre-fetched data)
    let workouts_for_date = app
        .service
        .list_workouts(&WorkoutFilters {
            date: Some(app.log_viewed_date),
            ..Default::default()
        })
        .unwrap_or_default(); // Handle error appropriately if needed
    app.update_log_sets_for_selected_exercise(&workouts_for_date); // Use the method from data.rs
}

pub fn log_list_previous(app: &mut App) {
    let current_selection = app.log_exercise_list_state.selected();
    let list_len = app.log_exercises_today.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.log_exercise_list_state.select(Some(i));
    let workouts_for_date = app
        .service
        .list_workouts(&WorkoutFilters {
            date: Some(app.log_viewed_date),
            ..Default::default()
        })
        .unwrap_or_default();
    app.update_log_sets_for_selected_exercise(&workouts_for_date);
}

pub fn log_table_next(app: &mut App) {
    let current_selection = app.log_set_table_state.selected();
    let list_len = app.log_sets_for_selected_exercise.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.log_set_table_state.select(Some(i));
}

pub fn log_table_previous(app: &mut App) {
    let current_selection = app.log_set_table_state.selected();
    let list_len = app.log_sets_for_selected_exercise.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.log_set_table_state.select(Some(i));
}

// --- Bodyweight Tab Navigation ---

pub fn bw_table_next(app: &mut App) {
    let current_selection = app.bw_history_state.selected();
    let list_len = app.bw_history.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    app.bw_history_state.select(Some(i));
}

pub fn bw_table_previous(app: &mut App) {
    let current_selection = app.bw_history_state.selected();
    let list_len = app.bw_history.len();
    if list_len == 0 {
        return;
    }
    let i = match current_selection {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    app.bw_history_state.select(Some(i));
}

// --- History Tab Navigation ---

pub fn history_list_next(app: &mut App) {
    navigation_helpers::list_next(&mut app.history_list_state, app.history_data.len());
}

pub fn history_list_previous(app: &mut App) {
    navigation_helpers::list_previous(&mut app.history_list_state, app.history_data.len());
}
