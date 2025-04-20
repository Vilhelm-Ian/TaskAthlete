use super::helpers::{
    render_button_pair, render_error_message, render_exercise_suggestions_popup,
    render_horizontal_input_pair, render_input_field,
};
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField},
        App,
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

         let height = calculate_workout_modal_height(error_message.is_some());
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

         // Edit modal might be slightly shorter as exercise name isn't an input field
         let height = calculate_workout_modal_height(error_message.is_some()) - 1;
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

/// Calculates the required height for the workout modal based on content.
fn calculate_workout_modal_height(has_error: bool) -> u16 {
    // Base: Exercise(1/2) + Sets/Reps(2) + Wt/Dur(2) + Dist(2) + NotesLabel(1) + NotesInput(3) + Spacer(1) + Buttons(1)
    // Edit mode saves 1 line as Exercise is just a label. Add mode may need suggestions.
    // Let's use a fixed reasonable base height and add error line if needed.
    14 + if has_error { 1 } else { 0 }
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
    let (weight_unit, dist_unit) = get_units(&app.service.config.units);

    let mut constraints = vec![
        Constraint::Length(1), // Exercise title/label
        Constraint::Length(if is_exercise_editable { 1 } else { 0 }), // Exercise input (optional height)
        Constraint::Length(2),                                        // Sets/Reps pair
        Constraint::Length(2),                                        // Weight/Duration pair
        Constraint::Length(2),                                        // Distance field
        Constraint::Length(1),                                        // Notes label
        Constraint::Length(3),                                        // Notes input
        Constraint::Length(1),                                        // Spacer
        Constraint::Length(1),                                        // Buttons row
    ];
    if error_message.is_some() {
        constraints.push(Constraint::Length(1));
    } // Error
    constraints.push(Constraint::Min(0)); // Fill remainder

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            constraints
                .iter()
                .filter(|&&c| c != Constraint::Length(0)) // Filter out zero height constraints
                .cloned()
                .collect::<Vec<_>>(),
        )
        .split(area);

    let mut current_chunk_index = 0;
    let mut input_areas = Vec::with_capacity(7); // Store text areas for cursor

    // --- Exercise ---
    f.render_widget(Paragraph::new(title_line), chunks[current_chunk_index]);
    current_chunk_index += 1;

    if is_exercise_editable {
        // The exercise title already exists, render input below it on the *next* chunk
        let exercise_text_area = render_input_field(
            f,
            chunks[current_chunk_index], // Use the dedicated input chunk
            "",                          // No separate label needed, title acts as label
            exercise_input,
            *focused_field == AddWorkoutField::Exercise
                || *focused_field == AddWorkoutField::Suggestions,
        );
        input_areas.push(exercise_text_area);
        current_chunk_index += 1;
    } else {
        input_areas.push(Rect::default()); // Placeholder rect if not editable
                                           // No increment needed for current_chunk_index as no chunk was used
    }

    // --- Sets/Reps ---
    let (sets_area, reps_area) = render_horizontal_input_pair(
        f,
        chunks[current_chunk_index],
        "Sets:",
        sets_input,
        *focused_field == AddWorkoutField::Sets,
        "Reps:",
        reps_input,
        *focused_field == AddWorkoutField::Reps,
    );
    input_areas.push(sets_area);
    input_areas.push(reps_area);
    current_chunk_index += 1;

    // --- Weight/Duration ---
    let weight_label_text = get_weight_label(resolved_exercise, weight_unit);
    let (weight_area, duration_area) = render_horizontal_input_pair(
        f,
        chunks[current_chunk_index],
        &weight_label_text,
        weight_input,
        *focused_field == AddWorkoutField::Weight,
        "Duration (min):",
        duration_input,
        *focused_field == AddWorkoutField::Duration,
    );
    input_areas.push(weight_area);
    input_areas.push(duration_area);
    current_chunk_index += 1;

    // --- Distance ---
    let distance_area = render_input_field(
        f,
        chunks[current_chunk_index],
        &format!("Distance ({dist_unit}):"),
        distance_input,
        *focused_field == AddWorkoutField::Distance,
    );
    input_areas.push(distance_area);
    current_chunk_index += 1;

    // --- Notes ---
    input_areas.push(render_notes_field(
        f,
        chunks[current_chunk_index],
        chunks[current_chunk_index + 1],
        notes_input,
        focused_field,
    ));
    current_chunk_index += 2; // Consumes label and input chunks

    // --- Buttons ---
    current_chunk_index += 1; // Skip Spacer
    let button_focus = match focused_field {
        AddWorkoutField::Confirm => Some(0),
        AddWorkoutField::Cancel => Some(1),
        _ => None,
    };
    render_button_pair(f, chunks[current_chunk_index], "OK", "Cancel", button_focus);
    current_chunk_index += 1;

    // --- Error ---
    if chunks.len() > current_chunk_index {
        render_error_message(f, chunks[current_chunk_index], error_message);
    }

    // Suggestions popup is rendered *after* this function returns, in the main modal renderer

    input_areas // Return calculated text areas
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
            // Only if editable
            let (x, y) = get_cursor_pos(exercise_input, &input_areas[0]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Sets => {
            let (x, y) = get_cursor_pos(sets_input, &input_areas[1]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Reps => {
            let (x, y) = get_cursor_pos(reps_input, &input_areas[2]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Weight => {
            let (x, y) = get_cursor_pos(weight_input, &input_areas[3]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Duration => {
            let (x, y) = get_cursor_pos(duration_input, &input_areas[4]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Distance => {
            let (x, y) = get_cursor_pos(distance_input, &input_areas[5]);
            f.set_cursor(x, y);
        }
        AddWorkoutField::Notes => {
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
        _ => {} // No cursor for Confirm/Cancel or non-editable Exercise
    }
}
