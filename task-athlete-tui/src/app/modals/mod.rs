// src/app/modals/mod.rs

// Declare the modules within the modals folder
mod add_workout;
mod confirm_delete_bodyweight;
mod confirm_delete_workout_set;
mod create_exercise;
mod edit_workout;
mod input_helpers;
mod log_bodyweight;
mod pb_modal;
mod set_target_weight;

// Re-export public input handler functions for use by the main app module
pub use add_workout::handle_add_workout_modal_input;
pub use confirm_delete_bodyweight::handle_confirm_delete_body_weigth_input;
pub use confirm_delete_workout_set::handle_confirm_delete_modal_input;
pub use create_exercise::handle_create_exercise_modal_input;
pub use edit_workout::handle_edit_workout_modal_input;
pub use log_bodyweight::handle_log_bodyweight_modal_input;
pub use pb_modal::handle_pb_modal_input;
pub use set_target_weight::handle_set_target_weight_modal_input;

// No need to re-export submit or parsing functions as they are generally internal
// to the modal logic, called by the handlers.
