// src/ui/modals.rs
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField, LogBodyweightField, SetTargetWeightField},
        AddExerciseField, App,
    },
    ui::layout::{centered_rect, centered_rect_fixed},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::{ExerciseDefinition, ExerciseType, Units};

// --- Rendering Helpers ---

/// Renders a labeled input field and returns the area used by the input paragraph itself.
fn render_input_field(
    f: &mut Frame,
    area: Rect, // The Rect allocated for this field (label + input line)
    label: &str,
    value: &str,
    is_focused: bool,
) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)]) // Label, Input
        .split(area); // Split the provided area

    f.render_widget(Paragraph::new(label), chunks[0]);

    let base_input_style = Style::default().fg(Color::White);
    let input_style = if is_focused {
        base_input_style.reversed()
    } else {
        base_input_style
    };
    let input_margin = Margin {
        vertical: 0,
        horizontal: 1,
    };
    let text_area = chunks[1].inner(&input_margin);
    f.render_widget(Paragraph::new(value).style(input_style), text_area);

    // Return the area where the text *value* is drawn (useful for cursor positioning)
    text_area
}

/// Renders a standard horizontal pair of buttons (e.g., OK/Cancel).
fn render_button_pair(
    f: &mut Frame,
    area: Rect,
    label1: &str,
    label2: &str,
    focused_button: Option<u8>, // 0 for first, 1 for second
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let base_style = Style::default().fg(Color::White);

    let style1 = if focused_button == Some(0) {
        base_style.reversed()
    } else {
        base_style
    };
    f.render_widget(
        Paragraph::new(format!(" {} ", label1))
            .alignment(ratatui::layout::Alignment::Center)
            .style(style1),
        chunks[0],
    );

    let style2 = if focused_button == Some(1) {
        base_style.reversed()
    } else {
        base_style
    };
    f.render_widget(
        Paragraph::new(format!(" {} ", label2))
            .alignment(ratatui::layout::Alignment::Center)
            .style(style2),
        chunks[1],
    );
}

/// Renders an optional error message line.
fn render_error_message(f: &mut Frame, area: Rect, error_message: Option<&String>) {
    if let Some(err) = error_message {
        f.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            area,
        );
    }
}

// --- Main Modal Rendering Logic ---

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f),
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
        ActiveModal::AddWorkout { .. } => render_add_workout_modal(f, app),
        ActiveModal::CreateExercise { .. } => render_create_exercise_modal(f, app),
        ActiveModal::EditWorkout { .. } => render_edit_workout_modal(f, app),
        ActiveModal::ConfirmDeleteWorkout { .. } => render_confirmation_modal(f, app),
        ActiveModal::None => {}
    }
}

// --- Specific Modal Renderers (Refactored) ---

fn render_help_modal(f: &mut Frame) {
    // (Keep existing help modal code - it's unique)
    let block = Block::default()
        .title("Help (?)")
        .borders(Borders::ALL)
        .title_style(Style::new().bold())
        .border_style(Style::new().yellow());
    let area = centered_rect(60, 70, f.size());
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let help_text = vec![
        Line::from("--- Global ---").style(Style::new().bold().underlined()),
        Line::from(" Q: Quit Application"),
        Line::from(" ?: Show/Hide This Help"),
        Line::from(" F1-F4: Switch Tabs"),
        Line::from(""),
        Line::from("--- Log Tab (F1) ---").style(Style::new().bold().underlined()),
        Line::from(" k / ↑: Navigate Up"),
        Line::from(" j / ↓: Navigate Down"),
        Line::from(" Tab: Switch Focus (Exercises List <=> Sets Table)"),
        Line::from(" h / ←: View Previous Day"),
        Line::from(" l / →: View Next Day"),
        Line::from(" a: Add New Workout Entry (for viewed day)"),
        Line::from(" c: Create New Exercise Definition"), // Updated
        Line::from(" e / Enter: Edit Selected Set/Entry (in Sets Table)"),
        Line::from(" d / Delete: Delete Selected Set/Entry (in Sets Table)"),
        Line::from(" g: Go to Graphs for Selected Exercise (TODO)"),
        Line::from(""),
        // ... (rest of help text remains the same) ...
        Line::from("--- Bodyweight Tab (F4) ---").style(Style::new().bold().underlined()),
        Line::from(" Tab: Cycle Focus (Graph, Actions, History) (TODO)"),
        Line::from(" k/j / ↑/↓: Navigate History Table (when focused)"),
        Line::from(" l: Log New Bodyweight Entry"),
        Line::from(" t: Set/Clear Target Bodyweight"),
        Line::from(" r: Cycle Graph Time Range (1M > 3M > 6M > 1Y > All)"),
        Line::from(""),
        Line::from(Span::styled(
            " Press Esc, ?, or Enter to close ",
            Style::new().italic().yellow(),
        )),
    ];

    let paragraph = Paragraph::new(help_text).wrap(Wrap { trim: false });
    f.render_widget(
        paragraph,
        area.inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}

fn render_log_bodyweight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::LogBodyweight {
        weight_input,
        date_input,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let block = Block::default()
            .title("Log New Bodyweight")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());
        // Calculate height based on content
        let height = 6 + if error_message.is_some() { 1 } else { 0 };
        let area = centered_rect_fixed(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Weight field (label + input)
            Constraint::Length(2), // Date field (label + input)
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let weight_text_area = render_input_field(
            f,
            chunks[0],
            &format!("Weight ({}):", weight_unit),
            weight_input,
            *focused_field == LogBodyweightField::Weight,
        );

        let date_text_area = render_input_field(
            f,
            chunks[1],
            "Date (YYYY-MM-DD / today/yesterday):",
            date_input,
            *focused_field == LogBodyweightField::Date,
        );

        let button_focus = match focused_field {
            LogBodyweightField::Confirm => Some(0),
            LogBodyweightField::Cancel => Some(1),
            _ => None,
        };
        render_button_pair(f, chunks[2], "OK", "Cancel", button_focus);

        let error_chunk_index = 3;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // --- Cursor Positioning ---
        match focused_field {
            LogBodyweightField::Weight => {
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            LogBodyweightField::Date => {
                let cursor_x = (date_text_area.x + date_input.chars().count() as u16)
                    .min(date_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, date_text_area.y);
            }
            _ => {}
        }
    }
}

fn render_set_target_weight_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::SetTargetWeight {
        weight_input,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let block = Block::default()
            .title("Set Target Bodyweight")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        let height = 5 + if error_message.is_some() { 1 } else { 0 };
        let area = centered_rect_fixed(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Target field (label + input)
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let weight_text_area = render_input_field(
            f,
            chunks[0],
            &format!("Target Weight ({}):", weight_unit),
            weight_input,
            *focused_field == SetTargetWeightField::Weight,
        );

        // Button Rendering (3 buttons)
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[1]); // Buttons in chunk 1

        let base_button_style = Style::default().fg(Color::White);
        f.render_widget(
            Paragraph::new(" Set ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(if *focused_field == SetTargetWeightField::Set {
                    base_button_style.reversed()
                } else {
                    base_button_style
                }),
            button_layout[0],
        );
        f.render_widget(
            Paragraph::new(" Clear Target ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(if *focused_field == SetTargetWeightField::Clear {
                    base_button_style.reversed()
                } else {
                    base_button_style
                }),
            button_layout[1],
        );
        f.render_widget(
            Paragraph::new(" Cancel ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(if *focused_field == SetTargetWeightField::Cancel {
                    base_button_style.reversed()
                } else {
                    base_button_style
                }),
            button_layout[2],
        );

        let error_chunk_index = 2;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // --- Cursor Positioning ---
        match focused_field {
            SetTargetWeightField::Weight => {
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            _ => {}
        }
    }
}

// Helper function to render a pair of input fields horizontally
fn render_horizontal_input_pair(
    f: &mut Frame,
    area: Rect,
    label1: &str,
    value1: &str,
    is_focused1: bool,
    label2: &str,
    value2: &str,
    is_focused2: bool,
) -> (Rect, Rect) {
    // Returns text areas for cursor positioning
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let text_area1 = render_input_field(f, chunks[0], label1, value1, is_focused1);
    let text_area2 = render_input_field(f, chunks[1], label2, value2, is_focused2);
    (text_area1, text_area2)
}

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
    exercise_suggestions: Option<&Vec<String>>, // Make optional
    suggestion_list_state: Option<&ListState>,  // Make optional
) -> Vec<Rect> // Return areas for cursor positioning if needed
{
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let dist_unit = match app.service.config.units {
        Units::Metric => "km",
        Units::Imperial => "mi",
    };

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
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            constraints
                .iter()
                .filter(|&&c| c != Constraint::Length(0))
                .cloned()
                .collect::<Vec<_>>(),
        ) // Filter out zero height constraints
        .split(area);

    let mut current_chunk_index = 0;
    let mut input_areas = Vec::with_capacity(6); // Store text areas for cursor

    // --- Exercise ---
    f.render_widget(Paragraph::new(title_line), chunks[current_chunk_index]);
    current_chunk_index += 1;
    if is_exercise_editable {
        let exercise_text_area = render_input_field(
            f,
            chunks[current_chunk_index - 1], // Reuse the title chunk if not editable? NO, use next chunk
            "",                              // No label, title serves as label
            exercise_input,
            *focused_field == AddWorkoutField::Exercise
                || *focused_field == AddWorkoutField::Suggestions,
        );
        input_areas.push(exercise_text_area);
        current_chunk_index += 1;
    } else {
        input_areas.push(Rect::default()); // Placeholder for exercise area index
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
    let weight_label_text =
        if resolved_exercise.map_or(false, |def| def.type_ == ExerciseType::BodyWeight) {
            format!("Added Weight ({}):", weight_unit)
        } else {
            format!("Weight ({}):", weight_unit)
        };
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
        &format!("Distance ({}):", dist_unit),
        distance_input,
        *focused_field == AddWorkoutField::Distance,
    );
    input_areas.push(distance_area);
    current_chunk_index += 1;

    // --- Notes ---
    f.render_widget(Paragraph::new("Notes:"), chunks[current_chunk_index]);
    current_chunk_index += 1;
    let notes_style = if *focused_field == AddWorkoutField::Notes {
        Style::default().fg(Color::White).reversed()
    } else {
        Style::default().fg(Color::White)
    };
    let notes_text_area = chunks[current_chunk_index].inner(&Margin {
        vertical: 0,
        horizontal: 1,
    });
    f.render_widget(
        Paragraph::new(notes_input)
            .wrap(Wrap { trim: false })
            .style(notes_style)
            .block(Block::default().borders(Borders::LEFT)), // Indent notes
        notes_text_area,
    );
    input_areas.push(notes_text_area); // Add notes area for cursor
    current_chunk_index += 1;

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

    // --- Render Suggestions Popup (if applicable) ---
    if is_exercise_editable
        && (*focused_field == AddWorkoutField::Exercise
            || *focused_field == AddWorkoutField::Suggestions)
    {
        if let (Some(suggestions), Some(list_state)) = (exercise_suggestions, suggestion_list_state)
        {
            if !suggestions.is_empty() {
                let suggestions_height = suggestions.len().min(5) as u16 + 2; // Limit height + borders
                let suggestions_width = input_areas[0].width; // Match input width
                let suggestions_x = input_areas[0].x;
                let suggestions_y = input_areas[0].y + 1; // Position below exercise input

                let popup_area = Rect {
                    x: suggestions_x,
                    y: suggestions_y,
                    width: suggestions_width.min(f.size().width.saturating_sub(suggestions_x)),
                    height: suggestions_height.min(f.size().height.saturating_sub(suggestions_y)),
                };

                let list_items: Vec<ListItem> = suggestions
                    .iter()
                    .map(|s| ListItem::new(s.as_str()))
                    .collect();
                let suggestions_list = List::new(list_items)
                    .block(Block::default().borders(Borders::ALL).title("Suggestions"))
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                f.render_widget(Clear, popup_area);
                let mut state = list_state.clone(); // Clone for rendering
                f.render_stateful_widget(suggestions_list, popup_area, &mut state);
            }
        }
    }

    input_areas // Return calculated text areas
}

fn render_add_workout_modal(f: &mut Frame, app: &App) {
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

         // More dynamic height calculation
         let base_height = 14; // Approximate base height for fields/labels/buttons
         let height = base_height + if error_message.is_some() { 1 } else { 0 };
         let area = centered_rect_fixed(80, height, f.size());

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

         // --- Cursor Positioning ---
         match focused_field {
             AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                 let cursor_x = (input_areas[0].x + exercise_input.chars().count() as u16)
                     .min(input_areas[0].right().saturating_sub(1));
                 f.set_cursor(cursor_x, input_areas[0].y);
             }
             AddWorkoutField::Sets => f.set_cursor(input_areas[1].x + sets_input.chars().count() as u16, input_areas[1].y),
             AddWorkoutField::Reps => f.set_cursor(input_areas[2].x + reps_input.chars().count() as u16, input_areas[2].y),
             AddWorkoutField::Weight => f.set_cursor(input_areas[3].x + weight_input.chars().count() as u16, input_areas[3].y),
             AddWorkoutField::Duration => f.set_cursor(input_areas[4].x + duration_input.chars().count() as u16, input_areas[4].y),
             AddWorkoutField::Distance => f.set_cursor(input_areas[5].x + distance_input.chars().count() as u16, input_areas[5].y),
             AddWorkoutField::Notes => {
                 let lines: Vec<&str> = notes_input.lines().collect();
                 let last_line = lines.last().unwrap_or(&"");
                 let notes_area = input_areas[6]; // Use the calculated notes area
                 let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
                 let cursor_x = notes_area.x + last_line.chars().count() as u16;
                 f.set_cursor(
                     cursor_x.min(notes_area.right().saturating_sub(1)), // Ensure cursor stays within bounds
                     cursor_y.min(notes_area.bottom().saturating_sub(1)),
                 );
             }
             _ => {}
         }
     }
}

fn render_edit_workout_modal(f: &mut Frame, app: &App) {
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

        // Adjust height because exercise field is not editable (takes less space visually)
         let base_height = 13;
         let height = base_height + if error_message.is_some() { 1 } else { 0 };
         let area = centered_rect_fixed(80, height, f.size());

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

         // --- Cursor Positioning (skip exercise field index 0) ---
         match focused_field {
             // Indexing shifted because exercise area is placeholder
             AddWorkoutField::Sets => f.set_cursor(input_areas[1].x + sets_input.chars().count() as u16, input_areas[1].y),
             AddWorkoutField::Reps => f.set_cursor(input_areas[2].x + reps_input.chars().count() as u16, input_areas[2].y),
             AddWorkoutField::Weight => f.set_cursor(input_areas[3].x + weight_input.chars().count() as u16, input_areas[3].y),
             AddWorkoutField::Duration => f.set_cursor(input_areas[4].x + duration_input.chars().count() as u16, input_areas[4].y),
             AddWorkoutField::Distance => f.set_cursor(input_areas[5].x + distance_input.chars().count() as u16, input_areas[5].y),
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
             _ => {}
         }
     }
}

fn render_create_exercise_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::CreateExercise {
        name_input,
        muscles_input,
        selected_type,
        focused_field,
        error_message,
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Create New Exercise")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        let height = 10 + if error_message.is_some() { 1 } else { 0 };
        let area = centered_rect_fixed(60, height, f.size());

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(2), // Name field
            Constraint::Length(2), // Muscles field
            Constraint::Length(1), // Type label
            Constraint::Length(1), // Type options
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1));
        } // Error
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let name_text_area = render_input_field(
            f,
            chunks[0],
            "Name:",
            name_input,
            *focused_field == AddExerciseField::Name,
        );
        let muscles_text_area = render_input_field(
            f,
            chunks[1],
            "Muscles (comma-separated):",
            muscles_input,
            *focused_field == AddExerciseField::Muscles,
        );

        // Type Label & Options
        f.render_widget(Paragraph::new("Type:"), chunks[2]);
        let type_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]); // Types in chunk 3

        let base_button_style = Style::default().fg(Color::White);
        let type_styles = [
            ExerciseType::Resistance,
            ExerciseType::Cardio,
            ExerciseType::BodyWeight,
        ]
        .iter()
        .map(|t| {
            if *selected_type == *t {
                base_button_style.bg(Color::DarkGray)
            } else {
                base_button_style
            }
        })
        .collect::<Vec<_>>();
        let type_focus_styles = [
            AddExerciseField::TypeResistance,
            AddExerciseField::TypeCardio,
            AddExerciseField::TypeBodyweight,
        ]
        .iter()
        .map(|ff| {
            if *focused_field == *ff {
                Modifier::REVERSED
            } else {
                Modifier::empty()
            }
        })
        .collect::<Vec<_>>();

        f.render_widget(
            Paragraph::new(" Resistance ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(type_styles[0].add_modifier(type_focus_styles[0])),
            type_layout[0],
        );
        f.render_widget(
            Paragraph::new(" Cardio ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(type_styles[1].add_modifier(type_focus_styles[1])),
            type_layout[1],
        );
        f.render_widget(
            Paragraph::new(" BodyWeight ")
                .alignment(ratatui::layout::Alignment::Center)
                .style(type_styles[2].add_modifier(type_focus_styles[2])),
            type_layout[2],
        );

        // Buttons
        let button_focus = match focused_field {
            AddExerciseField::Confirm => Some(0),
            AddExerciseField::Cancel => Some(1),
            _ => None,
        };
        render_button_pair(f, chunks[5], "OK", "Cancel", button_focus); // Buttons in chunk 5 (after spacer)

        // Error
        let error_chunk_index = 6;
        if chunks.len() > error_chunk_index {
            render_error_message(f, chunks[error_chunk_index], error_message.as_ref());
        }

        // Cursor Positioning
        match focused_field {
            AddExerciseField::Name => {
                let cursor_x = (name_text_area.x + name_input.chars().count() as u16)
                    .min(name_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, name_text_area.y);
            }
            AddExerciseField::Muscles => {
                let cursor_x = (muscles_text_area.x + muscles_input.chars().count() as u16)
                    .min(muscles_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, muscles_text_area.y);
            }
            _ => {}
        }
    }
}

fn render_confirmation_modal(f: &mut Frame, app: &App) {
    // (Keep existing confirmation modal code - it's simple and unique)
    if let ActiveModal::ConfirmDeleteWorkout {
        exercise_name,
        set_index,
        ..
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD));

        let question = format!("Delete set {} of {}?", set_index, exercise_name);
        let options = "[Y]es / [N]o (Esc)";

        let question_width = question.len() as u16;
        let options_width = options.len() as u16;
        let text_width = question_width.max(options_width);
        let modal_width = text_width + 4;
        let modal_height = 5;

        let area = centered_rect_fixed(modal_width, modal_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner_area);

        f.render_widget(
            Paragraph::new(question).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(options).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );
    }
}
