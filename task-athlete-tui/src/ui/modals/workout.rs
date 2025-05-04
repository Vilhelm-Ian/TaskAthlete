// src/ui/modals/workout.rs
use super::helpers::{
    render_button_pair, render_error_message, render_exercise_suggestions_popup,
    render_horizontal_input_pair, render_input_field,
};
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField, App, WorkoutLogFlags}, // Import WorkoutLogFlags
    },
    ui::layout::centered_rect,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Clear, ListState, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::{ExerciseDefinition, ExerciseType, Units};

// --- Main Render Functions ---

pub(super) fn render_add_workout_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::AddWorkout {
         exercise_input,
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         focused_field,
         error_message,
         resolved_exercise,
         exercise_suggestions,
         suggestion_list_state,
         .. // Ignore all_exercise_identifiers
     } = &app.active_modal {
         let block = Block::default()
             .title("Add New Workout Entry")
             .borders(Borders::ALL)
             .border_style(Style::new().yellow());

         let flags = WorkoutLogFlags::from_def(resolved_exercise.as_ref());
         let height = calculate_workout_modal_height(&flags, error_message.is_some());
         let area = centered_rect(80, height, f.size());

         f.render_widget(Clear, area);
         f.render_widget(block, area);

         let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });

         let input_areas = render_workout_modal_content(
             f, app, inner_area,
             "Exercise Name/Alias:".to_string(), true, // Editable
             exercise_input, sets_input, reps_input, weight_input, duration_input, distance_input, notes_input,
             focused_field, error_message.as_ref(), resolved_exercise.as_ref(),
             Some(exercise_suggestions), Some(suggestion_list_state)
         );

         position_cursor_for_workout(f, focused_field, exercise_input, sets_input, reps_input, weight_input, duration_input, distance_input, notes_input, &input_areas);

         // Render suggestions popup if needed (after positioning cursor for main input)
         if let (true, Some(suggestions), Some(list_state)) = (
             *focused_field == AddWorkoutField::Exercise || *focused_field == AddWorkoutField::Suggestions,
             Some(exercise_suggestions),
             Some(suggestion_list_state),
         ) {
             if !input_areas.is_empty() { // Ensure input_areas[0] exists
                 render_exercise_suggestions_popup(f, suggestions, list_state, input_areas[0]);
             }
         }
     }
}

pub(super) fn render_edit_workout_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::EditWorkout {
         exercise_name,
         sets_input,
         reps_input,
         weight_input,
         duration_input,
         distance_input,
         notes_input,
         focused_field,
         error_message,
         resolved_exercise,
         .. // workout_id not rendered, no suggestions
     } = &app.active_modal {
         let block = Block::default()
             .title(format!("Edit Workout Entry ({})", exercise_name))
             .borders(Borders::ALL)
             .border_style(Style::new().yellow());

         let flags = WorkoutLogFlags::from_def(resolved_exercise.as_ref());
         let height = calculate_workout_modal_height(&flags, error_message.is_some());
         let area = centered_rect(80, height, f.size());

         f.render_widget(Clear, area);
         f.render_widget(block, area);

         let inner_area = area.inner(&Margin { vertical: 1, horizontal: 1 });

         let input_areas = render_workout_modal_content(
             f, app, inner_area,
             format!("Exercise: {}", exercise_name), false, // Not editable
             "", // Exercise input value not needed here
             sets_input, reps_input, weight_input, duration_input, distance_input, notes_input,
             focused_field, error_message.as_ref(), resolved_exercise.as_ref(),
             None, None // No suggestions needed for edit modal
         );

         position_cursor_for_workout(f, focused_field, "", sets_input, reps_input, weight_input, duration_input, distance_input, notes_input, &input_areas);
     }
}

// --- Shared Rendering Logic ---

/// Calculates the required height dynamically based on visible fields.
fn calculate_workout_modal_height(flags: &WorkoutLogFlags, has_error: bool) -> u16 {
    let mut height = 0;
    height += 1; // Exercise title/label
    height += 1; // Exercise input (always reserve space, even if read-only label)

    // Combined Sets/Reps row
    if flags.log_sets || flags.log_reps {
        height += 2;
    }
    // Combined Weight/Duration row
    if flags.log_weight || flags.log_duration {
        height += 2;
    }
    // Distance row
    if flags.log_distance {
        height += 2;
    }
    // Notes
    if flags.log_notes {
        height += 1; // Label
        height += 3; // Input area
    }
    height += 1; // Spacer
    height += 1; // Buttons
    if has_error {
        height += 1;
    }
    height += 2; // Modal top/bottom borders

    height
}

/// Renders the common fields for Add/Edit Workout modals.
/// Returns a Vec of Rects corresponding to the *text input areas* for cursor positioning.
/// Indices: 0:Exercise(or dummy), 1:Sets, 2:Reps, 3:Weight, 4:Duration, 5:Distance, 6:Notes
fn render_workout_modal_content(
    f: &mut Frame,
    app: &App,
    area: Rect,         // Inner area after block
    title_line: String, // e.g., "Exercise: Bench Press" or "Exercise Name/Alias:"
    is_exercise_editable: bool,
    exercise_input: &str,
    sets_input: &str,
    reps_input: &str,
    weight_input: &str,
    duration_input: &str,
    distance_input: &str,
    notes_input: &str,
    focused_field: &AddWorkoutField,
    error_message: Option<&String>,
    resolved_exercise: Option<&ExerciseDefinition>,
    _exercise_suggestions: Option<&Vec<String>>, // Handled separately now
    _suggestion_list_state: Option<&ListState>,  // Handled separately now
) -> Vec<Rect> {
    let flags = WorkoutLogFlags::from_def(resolved_exercise); // Get flags
    let (weight_unit, dist_unit) = get_units(&app.service.config.units);

    // --- Dynamically build constraints based on flags ---
    let mut constraints = vec![
        Constraint::Length(1), // Exercise title/label always present
        Constraint::Length(if is_exercise_editable { 1 } else { 0 }),
    ];
    if flags.log_sets || flags.log_reps {
        constraints.push(Constraint::Length(2)); // Sets/Reps pair
    }
    if flags.log_weight || flags.log_duration {
        constraints.push(Constraint::Length(2)); // Weight/Duration pair
    }
    if flags.log_distance {
        constraints.push(Constraint::Length(2)); // Distance field
    }
    if flags.log_notes {
        constraints.push(Constraint::Length(1)); // Notes label
        constraints.push(Constraint::Length(3)); // Notes input
    }
    constraints.push(Constraint::Length(1)); // Spacer
    constraints.push(Constraint::Length(1)); // Buttons row
    if error_message.is_some() {
        constraints.push(Constraint::Length(1)); // Error
    }
    constraints.push(Constraint::Min(0)); // Fill remainder

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            constraints
                .into_iter()
                .filter(|&c| c != Constraint::Length(0)) // Filter out zero height constraints
                .collect::<Vec<_>>(),
        )
        .split(area);

    // --- Render fields conditionally, tracking chunk index ---
    let mut current_chunk_index = 0;
    // Store results in fixed-size array or vec, using default Rect for hidden fields
    let mut input_areas = vec![Rect::default(); 7]; // 0:Ex, 1:Sets, 2:Reps, 3:Wt, 4:Dur, 5:Dist, 6:Notes

    // Exercise
    f.render_widget(Paragraph::new(title_line), chunks[current_chunk_index]);
    current_chunk_index += 1;
    if is_exercise_editable {
        let exercise_text_area = render_input_field(
            f,
            chunks[current_chunk_index], // Use the dedicated input chunk
            "",                          // No separate label needed, title acts as label
            exercise_input,
            *focused_field == AddWorkoutField::Exercise
                || *focused_field == AddWorkoutField::Suggestions,
        );
        input_areas[0] = exercise_text_area; // Store area
        current_chunk_index += 1;
    } // Else: input_areas[0] remains default

    // Sets/Reps Pair
    if flags.log_sets || flags.log_reps {
        let chunk = chunks[current_chunk_index];
        current_chunk_index += 1;
        let (sets_area, reps_area) = if flags.log_sets && flags.log_reps {
            render_horizontal_input_pair(
                f,
                chunk,
                "Sets:",
                sets_input,
                *focused_field == AddWorkoutField::Sets,
                "Reps:",
                reps_input,
                *focused_field == AddWorkoutField::Reps,
            )
        } else if flags.log_sets {
            // Only Sets visible
            let area = render_input_field(
                f,
                chunk,
                "Sets:",
                sets_input,
                *focused_field == AddWorkoutField::Sets,
            );
            (area, Rect::default())
        } else {
            // Only Reps visible
            let area = render_input_field(
                f,
                chunk,
                "Reps:",
                reps_input,
                *focused_field == AddWorkoutField::Reps,
            );
            (Rect::default(), area)
        };
        input_areas[1] = sets_area;
        input_areas[2] = reps_area;
    }

    // Weight/Duration Pair
    if flags.log_weight || flags.log_duration {
        let chunk = chunks[current_chunk_index];
        current_chunk_index += 1;
        let weight_label_text = get_weight_label(resolved_exercise, weight_unit);
        let (weight_area, duration_area) = if flags.log_weight && flags.log_duration {
            render_horizontal_input_pair(
                f,
                chunk,
                &weight_label_text,
                weight_input,
                *focused_field == AddWorkoutField::Weight,
                "Duration (min):",
                duration_input,
                *focused_field == AddWorkoutField::Duration,
            )
        } else if flags.log_weight {
            let area = render_input_field(
                f,
                chunk,
                &weight_label_text,
                weight_input,
                *focused_field == AddWorkoutField::Weight,
            );
            (area, Rect::default())
        } else {
            let area = render_input_field(
                f,
                chunk,
                "Duration (min):",
                duration_input,
                *focused_field == AddWorkoutField::Duration,
            );
            (Rect::default(), area)
        };
        input_areas[3] = weight_area;
        input_areas[4] = duration_area;
    }

    // Distance
    if flags.log_distance {
        let chunk = chunks[current_chunk_index];
        current_chunk_index += 1;
        let distance_area = render_input_field(
            f,
            chunk,
            &format!("Distance ({dist_unit}):"),
            distance_input,
            *focused_field == AddWorkoutField::Distance,
        );
        input_areas[5] = distance_area;
    }

    // Notes
    if flags.log_notes {
        let label_chunk = chunks[current_chunk_index];
        current_chunk_index += 1;
        let input_chunk = chunks[current_chunk_index];
        current_chunk_index += 1;
        input_areas[6] =
            render_notes_field(f, label_chunk, input_chunk, notes_input, focused_field);
    }

    // Spacer
    current_chunk_index += 1;

    // Buttons
    let button_focus = match focused_field {
        AddWorkoutField::Confirm => Some(0),
        AddWorkoutField::Cancel => Some(1),
        _ => None,
    };
    render_button_pair(f, chunks[current_chunk_index], "OK", "Cancel", button_focus);
    current_chunk_index += 1;

    // Error Message
    if chunks.len() > current_chunk_index {
        render_error_message(f, chunks[current_chunk_index], error_message);
    }

    input_areas
}

/// Helper to get the appropriate weight and distance units based on config.
fn get_units(units: &Units) -> (&str, &str) {
    match units {
        Units::Metric => ("kg", "km"),
        Units::Imperial => ("lbs", "mi"),
    }
}

/// Helper to determine the correct label for the weight field.
fn get_weight_label(resolved_exercise: Option<&ExerciseDefinition>, weight_unit: &str) -> String {
    if resolved_exercise.map_or(false, |def| def.type_ == ExerciseType::BodyWeight) {
        format!("Added Weight ({weight_unit}):")
    } else {
        format!("Weight ({weight_unit}):")
    }
}

/// Renders the notes label and input field. Returns the Rect of the text input area.
fn render_notes_field(
    f: &mut Frame,
    label_area: Rect,
    input_area: Rect,
    notes_input: &str,
    focused_field: &AddWorkoutField,
) -> Rect {
    f.render_widget(Paragraph::new("Notes:"), label_area);

    let notes_style = if *focused_field == AddWorkoutField::Notes {
        Style::default().fg(Color::White).reversed()
    } else {
        Style::default().fg(Color::White)
    };
    // Add a small margin for the notes input and a visual indicator (like border)
    let notes_text_area = input_area.inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });
    f.render_widget(
        Paragraph::new(notes_input)
            .wrap(Wrap { trim: false })
            .style(notes_style)
            .block(Block::default().borders(Borders::LEFT)), // Indent notes slightly
        notes_text_area,
    );
    notes_text_area // Return the actual drawable area
}

/// Helper to position the cursor within the Add/Edit Workout modal fields.
fn position_cursor_for_workout(
    f: &mut Frame,
    focused_field: &AddWorkoutField,
    exercise_input: &str,
    sets_input: &str,
    reps_input: &str,
    weight_input: &str,
    duration_input: &str,
    distance_input: &str,
    notes_input: &str,
    input_areas: &[Rect], // Expecting 7 areas (index 0 might be dummy)
) {
    if input_areas.len() < 7 {
        return;
    } // Safety check

    let get_cursor_pos = |input: &str, area: &Rect| -> (u16, u16) {
        let cursor_x = (area.x + input.chars().count() as u16).min(area.right().saturating_sub(1));
        (cursor_x, area.y)
    };

    match focused_field {
        AddWorkoutField::Exercise | AddWorkoutField::Suggestions if !input_areas[0].is_empty() => {
            // Only if editable and area is valid
            let (x, y) = get_cursor_pos(exercise_input, &input_areas[0]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Sets if !input_areas[1].is_empty() => {
            let (x, y) = get_cursor_pos(sets_input, &input_areas[1]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Reps if !input_areas[2].is_empty() => {
            let (x, y) = get_cursor_pos(reps_input, &input_areas[2]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Weight if !input_areas[3].is_empty() => {
            let (x, y) = get_cursor_pos(weight_input, &input_areas[3]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Duration if !input_areas[4].is_empty() => {
            let (x, y) = get_cursor_pos(duration_input, &input_areas[4]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Distance if !input_areas[5].is_empty() => {
            let (x, y) = get_cursor_pos(distance_input, &input_areas[5]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Notes if !input_areas[6].is_empty() => {
            let lines: Vec<&str> = notes_input.lines().collect();
            let last_line = lines.last().unwrap_or(&"");
            let notes_area = input_areas[6];
            let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
            let cursor_x = notes_area.x + last_line.chars().count() as u16;
            f.set_cursor(
                cursor_x.min(notes_area.right().saturating_sub(1)),
                cursor_y.min(notes_area.bottom().saturating_sub(1)),
            );
        }
        _ => {} // No cursor for Confirm/Cancel or hidden/non-editable fields
    }
}
