mod confirmation;
mod create_exercise;
mod help;
mod helpers; // Keep helpers private to the modals module unless needed elsewhere
mod log_bodyweight;
mod pb_modal;
mod set_target_weight;
mod workout;

use crate::app::state::ActiveModal;
use crate::app::App;
use ratatui::Frame;

/// Renders the currently active modal, if any.
pub fn render(f: &mut Frame, app: &App) {
    // Dispatch rendering to the appropriate function based on the active modal
    match &app.active_modal {
        ActiveModal::Help => help::render_help_modal(f),
        ActiveModal::LogBodyweight { .. } => log_bodyweight::render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => {
            set_target_weight::render_set_target_weight_modal(f, app)
        }
        ActiveModal::AddWorkout { .. } => workout::render_add_workout_modal(f, app),
        ActiveModal::CreateExercise { .. } => create_exercise::render_create_exercise_modal(f, app),
        ActiveModal::EditWorkout { .. } => workout::render_edit_workout_modal(f, app),
        ActiveModal::ConfirmDeleteWorkout { .. } => confirmation::render_confirmation_modal(f, app),
        ActiveModal::PersonalBest { .. } => pb_modal::render(f, app),
        ActiveModal::ConfirmDeleteBodyWeight { .. } => {
            confirmation::render_confirmation_bodyweight_modal(f, app);
        }
        ActiveModal::None => {} // Do nothing if no modal is active
    }
}
