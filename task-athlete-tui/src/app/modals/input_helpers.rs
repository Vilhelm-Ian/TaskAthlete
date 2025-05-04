// src/app/modals/input_helpers.rs
use crate::app::state::{AddWorkoutField, WorkoutLogFlags};

#[derive(PartialEq, Eq)]
pub(super) enum NavigationDirection {
    Forward,
    Backward,
}

// Order of fields for navigation purposes
const FOCUS_ORDER: &[AddWorkoutField] = &[
    AddWorkoutField::Exercise, // Only focusable in Add mode
    AddWorkoutField::Sets,
    AddWorkoutField::Reps,
    AddWorkoutField::Weight,
    AddWorkoutField::Duration,
    AddWorkoutField::Distance,
    AddWorkoutField::Notes,
    AddWorkoutField::Confirm,
    AddWorkoutField::Cancel,
];

// Fields that depend on flags and the function to check the flag
const FLAG_DEPENDENT_FIELDS: &[(AddWorkoutField, fn(&WorkoutLogFlags) -> bool)] = &[
    (AddWorkoutField::Reps, |f| f.log_reps),
    (AddWorkoutField::Weight, |f| f.log_weight),
    (AddWorkoutField::Duration, |f| f.log_duration),
    (AddWorkoutField::Distance, |f| f.log_distance),
    // Sets and Notes are handled slightly differently (assume always visible if needed)
];

/// Finds the next focusable field, skipping fields based on flags.
/// Note: Does not handle the Suggestions state transition.
pub(super) fn get_next_focusable_field(
    current: AddWorkoutField,
    flags: &WorkoutLogFlags,
    direction: NavigationDirection,
    is_add_mode: bool, // Need to know if Exercise field is potentially focusable
) -> AddWorkoutField {
    let order = FOCUS_ORDER;
    let current_index = order.iter().position(|&f| f == current).unwrap_or(0);
    let len = order.len();

    let mut next_index = if direction == NavigationDirection::Forward {
        (current_index + 1) % len
    } else {
        (current_index + len - 1) % len
    };

    // Iterate until a valid, focusable field is found
    for _ in 0..len {
        let next_field = order[next_index];

        // Check basic focusability (e.g., Exercise only in Add mode)
        let is_generally_focusable = match next_field {
            AddWorkoutField::Exercise => is_add_mode,
            AddWorkoutField::Suggestions => false, // Suggestions handled separately
            _ => true,
        };

        if is_generally_focusable {
            // Check visibility based on flags
            let is_visible = match next_field {
                AddWorkoutField::Sets => flags.log_sets, // Check sets flag (assuming always true for now)
                AddWorkoutField::Reps => flags.log_reps,
                AddWorkoutField::Weight => flags.log_weight,
                AddWorkoutField::Duration => flags.log_duration,
                AddWorkoutField::Distance => flags.log_distance,
                AddWorkoutField::Notes => flags.log_notes, // Check notes flag (assuming always true)
                // Exercise, Confirm, Cancel are always considered "visible" if generally focusable
                _ => true,
            };

            if is_visible {
                return next_field; // Found a valid, visible, focusable field
            }
        }

        // Move to the next index in the chosen direction
        next_index = if direction == NavigationDirection::Forward {
            (next_index + 1) % len
        } else {
            (next_index + len - 1) % len
        };

        // Prevent infinite loop if somehow no field is focusable (shouldn't happen)
        if next_index == current_index {
            break;
        }
    }

    current // Fallback: return current field if nothing else found
}
