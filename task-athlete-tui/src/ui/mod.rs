mod bodyweight_tab;
mod layout;
mod log_tab;
mod modals;
mod placeholders;
mod status_bar;
mod tabs;

// Re-export the main render function
pub use layout::render_ui; // Assuming render_ui is moved to layout.rs or stays here

//src/ui/modals.rs
// task-athlete-tui/src/ui/modals.rs
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField, LogBodyweightField, SetTargetWeightField},
        AddExerciseField, App,
    }, // Use App from crate::app
    ui::layout::centered_rect, // Use centered_rect from layout
    ui::layout::centered_rect_fixed,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::{ExerciseType, Units};

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f), // Don't need app state for help text
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
        ActiveModal::AddWorkout { .. } => render_add_workout_modal(f, app),
        ActiveModal::CreateExercise { .. } => render_create_exercise_modal(f, app),
        ActiveModal::EditWorkout { .. } => render_edit_workout_modal(f, app),
        ActiveModal::ConfirmDeleteWorkout { .. } => render_confirmation_modal(f, app),
        ActiveModal::None => {} // Should not happen if called correctly
    }
}

fn render_help_modal(f: &mut Frame) {
    // Removed unused `_app`
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
        Line::from(" a: Add New Workout Entry (for viewed day) (TODO)"),
        Line::from(" l: Log New Set (for selected exercise) (TODO)"),
        Line::from(" e / Enter: Edit Selected Set/Entry (TODO)"),
        Line::from(" d / Delete: Delete Selected Set/Entry (TODO)"),
        Line::from(" g: Go to Graphs for Selected Exercise (TODO)"),
        Line::from(""),
        Line::from("--- History Tab (F2) ---").style(Style::new().bold().underlined()),
        Line::from(" k/j / ↑/↓: Scroll History"),
        Line::from(" PgUp/PgDown: Scroll History Faster (TODO)"),
        Line::from(" / or f: Activate Filter Mode (TODO)"),
        Line::from(" e / Enter: Edit Selected Workout (TODO)"),
        Line::from(" d / Delete: Delete Selected Workout (TODO)"),
        Line::from(" Esc: Clear Filter / Exit Filter Mode (TODO)"),
        Line::from(""),
        Line::from("--- Graphs Tab (F3) ---").style(Style::new().bold().underlined()),
        Line::from(" Tab: Switch Focus (Selections) (TODO)"),
        Line::from(" k/j / ↑/↓: Navigate Selection List (TODO)"),
        Line::from(" Enter: Confirm Selection (TODO)"),
        Line::from(" /: Filter Exercise List (TODO)"),
        Line::from(""),
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
        let area = centered_rect(60, 11, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        // Get the inner area *after* the block's margin/border
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            // No margin here, use inner_area directly
            .constraints([
                Constraint::Length(1), // Weight label
                Constraint::Length(1), // Weight input
                Constraint::Length(1), // Date label
                Constraint::Length(1), // Date input
                Constraint::Length(1), // Spacer/Buttons row
                Constraint::Length(1), // Error Message (if any) - adjusted constraints
                Constraint::Min(0),    // Remaining space (might not be needed)
            ])
            .split(inner_area); // Split the inner_area

        f.render_widget(
            Paragraph::new(format!("Weight ({}):", weight_unit)),
            chunks[0],
        );
        f.render_widget(Paragraph::new("Date (YYYY-MM-DD / today):"), chunks[2]);

        // --- Input Field Rendering with Padding ---
        let base_input_style = Style::default().fg(Color::White); // Or another visible color

        // Weight Input
        let weight_input_area = chunks[1]; // Area for the whole line
                                           // Create a padded area *within* this line for the text itself
        let weight_text_area = weight_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let weight_style = if *focused_field == LogBodyweightField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        // Render the paragraph within the padded text_area
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_text_area,
        );

        // Date Input
        let date_input_area = chunks[3]; // Area for the whole line
                                         // Create a padded area *within* this line for the text itself
        let date_text_area = date_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let date_style = if *focused_field == LogBodyweightField::Date {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        // Render the paragraph within the padded text_area
        f.render_widget(
            Paragraph::new(date_input.as_str()).style(date_style),
            date_text_area,
        );
        // --- End Input Field Rendering ---

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[4]); // Buttons in chunk 4

        let base_button_style = Style::default().fg(Color::White);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[5],
            ); // Error in chunk 5
        }

        // --- Cursor Positioning (using padded areas) ---
        match focused_field {
            LogBodyweightField::Weight => {
                // Calculate cursor position relative to the padded weight_text_area
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1)); // Clamp to padded area
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            LogBodyweightField::Date => {
                // Calculate cursor position relative to the padded date_text_area
                let cursor_x = (date_text_area.x + date_input.chars().count() as u16)
                    .min(date_text_area.right().saturating_sub(1)); // Clamp to padded area
                f.set_cursor(cursor_x, date_text_area.y);
            }
            _ => {}
        }
        // --- End Cursor Positioning ---
    }
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
        exercise_suggestions, // Get suggestions
        suggestion_list_state, // Get list state
                              // all_exercise_identifiers is not needed for rendering
        ..
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Add New Workout Entry")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        // --- Calculate required height (including potential suggestions) ---
        let mut required_height = 2; // Borders/Padding
        required_height += 1; // Exercise label
        required_height += 1; // Exercise input
        required_height += 1; // Sets/Reps labels
        required_height += 1; // Sets/Reps inputs
        required_height += 1; // Weight/Duration labels
        required_height += 1; // Weight/Duration inputs
        required_height += 1; // Distance label
        required_height += 1; // Distance input
        required_height += 1; // Notes label
        required_height += 3; // Notes input (multi-line)
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error Message
        }
        // Note: We don't add suggestion height here, we'll draw it as a popup *over* other content

        let fixed_width = 80; // Keep width fixed
        let area = centered_rect_fixed(fixed_width, required_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        // Define constraints without suggestions initially
        let mut constraints = vec![
            Constraint::Length(1), // Exercise label
            Constraint::Length(1), // Exercise input
            Constraint::Length(1), // Sets/Reps labels
            Constraint::Length(1), // Sets/Reps inputs
            Constraint::Length(1), // Weight/Duration labels
            Constraint::Length(1), // Weight/Duration inputs
            Constraint::Length(1), // Distance label
            Constraint::Length(1), // Distance input
            Constraint::Length(1), // Notes label
            Constraint::Length(3), // Notes input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0)); // Remainder

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        // --- Render Main Modal Content (Mostly Unchanged) ---
        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };

        // Row 1: Exercise Input
        f.render_widget(Paragraph::new("Exercise Name/Alias:"), chunks[0]);
        let ex_style = if *focused_field == AddWorkoutField::Exercise
            || *focused_field == AddWorkoutField::Suggestions
        {
            // Highlight input if suggestions are focused too
            base_input_style.reversed()
        } else {
            base_input_style
        };
        let exercise_input_area = chunks[1].inner(&input_margin);
        f.render_widget(
            Paragraph::new(exercise_input.as_str()).style(ex_style),
            exercise_input_area,
        );

        // ... (Render Sets/Reps, Weight/Duration, Distance, Notes, Buttons, Error - unchanged logic, adjust chunk indices if error exists) ...
        let error_chunk_index = if error_message.is_some() {
            chunks.len() - 2
        } else {
            chunks.len() - 1
        }; // Error is before Min(0)
        let button_chunk_index = if error_message.is_some() {
            error_chunk_index - 1
        } else {
            error_chunk_index
        }; // Buttons are before error (or Min(0))
        let notes_chunk_index = button_chunk_index - 2; // Notes area is before spacer and buttons
        let notes_label_chunk_index = notes_chunk_index - 1;
        let distance_input_chunk_index = notes_label_chunk_index - 1;
        let distance_label_chunk_index = distance_input_chunk_index - 1;
        let weight_dur_inputs_chunk_index = distance_label_chunk_index - 1;
        let weight_dur_label_chunk_index = weight_dur_inputs_chunk_index - 1;
        let sets_reps_inputs_chunk_index = weight_dur_label_chunk_index - 1;
        let sets_reps_label_chunk_index = sets_reps_inputs_chunk_index - 1;

        // Row 2: Sets/Reps Labels
        let sets_reps_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_label_chunk_index]);
        f.render_widget(Paragraph::new("Sets:"), sets_reps_layout[0]);
        f.render_widget(Paragraph::new("Reps:"), sets_reps_layout[1]);
        // Row 2: Sets/Reps Inputs
        let sets_reps_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_inputs_chunk_index]);
        let sets_style = if *focused_field == AddWorkoutField::Sets {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(sets_input.as_str()).style(sets_style),
            sets_reps_inputs[0].inner(&input_margin),
        );
        let reps_style = if *focused_field == AddWorkoutField::Reps {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(reps_input.as_str()).style(reps_style),
            sets_reps_inputs[1].inner(&input_margin),
        );

        // Row 3: Weight/Duration Labels
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let weight_label_text = if resolved_exercise
            .as_ref()
            .map_or(false, |def| def.type_ == ExerciseType::BodyWeight)
        {
            format!("Added Weight ({}):", weight_unit)
        } else {
            format!("Weight ({}):", weight_unit)
        };
        let weight_dur_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_label_chunk_index]);
        f.render_widget(Paragraph::new(weight_label_text), weight_dur_layout[0]);
        f.render_widget(Paragraph::new("Duration (min):"), weight_dur_layout[1]);
        // Row 3: Weight/Duration Inputs
        let weight_dur_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_inputs_chunk_index]);
        let weight_style = if *focused_field == AddWorkoutField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_dur_inputs[0].inner(&input_margin),
        );
        let dur_style = if *focused_field == AddWorkoutField::Duration {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(duration_input.as_str()).style(dur_style),
            weight_dur_inputs[1].inner(&input_margin),
        );

        // Row 4: Distance Label & Input
        let dist_unit = match app.service.config.units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        f.render_widget(
            Paragraph::new(format!("Distance ({}):", dist_unit)),
            chunks[distance_label_chunk_index],
        );
        let dist_style = if *focused_field == AddWorkoutField::Distance {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(distance_input.as_str()).style(dist_style),
            chunks[distance_input_chunk_index].inner(&input_margin),
        );

        // Row 5: Notes Label & Input
        f.render_widget(Paragraph::new("Notes:"), chunks[notes_label_chunk_index]);
        let notes_style = if *focused_field == AddWorkoutField::Notes {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(notes_input.as_str())
                .wrap(Wrap { trim: false })
                .style(notes_style)
                .block(Block::default().borders(Borders::LEFT)),
            chunks[notes_chunk_index].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );

        // Row 6: Buttons
        let base_button_style = Style::default().fg(Color::White);
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);
        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 7: Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // --- Render Suggestions Popup ---
        if (*focused_field == AddWorkoutField::Exercise
            || *focused_field == AddWorkoutField::Suggestions)
            && !exercise_suggestions.is_empty()
        {
            let suggestions_height = exercise_suggestions.len() as u16 + 2; // +2 for border
            let suggestions_width = chunks[1].width; // Match input width
            let suggestions_x = chunks[1].x;
            // Position below the input field
            let suggestions_y = chunks[1].y + 1;

            // Create the popup area, ensuring it doesn't go off-screen
            let popup_area = Rect {
                x: suggestions_x,
                y: suggestions_y,
                width: suggestions_width.min(f.size().width.saturating_sub(suggestions_x)),
                height: suggestions_height.min(f.size().height.saturating_sub(suggestions_y)),
            };

            // Convert suggestions to ListItems
            let list_items: Vec<ListItem> = exercise_suggestions
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();

            // Create the list widget
            let suggestions_list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title("Suggestions"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            // Render the popup
            f.render_widget(Clear, popup_area); // Clear background under popup
                                                // Render statefully using a mutable clone of the state
            let mut list_state = suggestion_list_state.clone(); // Clone state for rendering
            f.render_stateful_widget(suggestions_list, popup_area, &mut list_state);
        }

        // --- Cursor Positioning ---
        match focused_field {
            // Position cursor in input field even when suggestions are focused
            AddWorkoutField::Exercise | AddWorkoutField::Suggestions => {
                let cursor_x = (exercise_input_area.x + exercise_input.chars().count() as u16)
                    .min(exercise_input_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, exercise_input_area.y);
            }
            AddWorkoutField::Sets => f.set_cursor(
                sets_reps_inputs[0].x + 1 + sets_input.chars().count() as u16,
                sets_reps_inputs[0].y,
            ),
            AddWorkoutField::Reps => f.set_cursor(
                sets_reps_inputs[1].x + 1 + reps_input.chars().count() as u16,
                sets_reps_inputs[1].y,
            ),
            AddWorkoutField::Weight => f.set_cursor(
                weight_dur_inputs[0].x + 1 + weight_input.chars().count() as u16,
                weight_dur_inputs[0].y,
            ),
            AddWorkoutField::Duration => f.set_cursor(
                weight_dur_inputs[1].x + 1 + duration_input.chars().count() as u16,
                weight_dur_inputs[1].y,
            ),
            AddWorkoutField::Distance => f.set_cursor(
                chunks[distance_input_chunk_index].x + 1 + distance_input.chars().count() as u16,
                chunks[distance_input_chunk_index].y,
            ),
            AddWorkoutField::Notes => {
                let lines: Vec<&str> = notes_input.lines().collect();
                let last_line = lines.last().unwrap_or(&"");
                let notes_area = chunks[notes_chunk_index].inner(&Margin {
                    vertical: 0,
                    horizontal: 1,
                }); // Area inside border
                let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
                let cursor_x = notes_area.x + last_line.chars().count() as u16;
                f.set_cursor(
                    cursor_x.min(notes_area.right() - 1),
                    cursor_y.min(notes_area.bottom() - 1),
                );
            }
            _ => {} // No cursor for buttons
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
        let area = centered_rect(60, 11, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        // Get the inner area *after* the block's margin/border
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            // No margin here, use inner_area directly
            .constraints([
                Constraint::Length(1), // Target label
                Constraint::Length(1), // Target input
                Constraint::Length(1), // Spacer/Buttons row
                Constraint::Length(1), // Buttons row
                Constraint::Length(1), // Error Message (if any) - adjusted constraints
                Constraint::Min(0),    // Remaining space
            ])
            .split(inner_area); // Split the inner_area

        f.render_widget(
            Paragraph::new(format!("Target Weight ({}):", weight_unit)),
            chunks[0],
        );

        // --- Input Field Rendering with Padding ---
        let base_input_style = Style::default().fg(Color::White);
        let weight_input_area = chunks[1];
        let weight_text_area = weight_input_area.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });
        let weight_style = if *focused_field == SetTargetWeightField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_text_area,
        );
        // --- End Input Field Rendering ---

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]); // Buttons in chunk 3

        let base_button_style = Style::default().fg(Color::White);
        let set_button = Paragraph::new(" Set ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Set {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(set_button, button_layout[0]);

        let clear_button = Paragraph::new(" Clear Target ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Clear {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(clear_button, button_layout[1]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[2]);

        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[4],
            ); // Error in chunk 4
        }

        // --- Cursor Positioning (using padded area) ---
        match focused_field {
            SetTargetWeightField::Weight => {
                let cursor_x = (weight_text_area.x + weight_input.chars().count() as u16)
                    .min(weight_text_area.right().saturating_sub(1));
                f.set_cursor(cursor_x, weight_text_area.y);
            }
            _ => {}
        }
        // --- End Cursor Positioning ---
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

        // --- Calculate Fixed Height ---
        let mut required_height = 2; // Top/Bottom border/padding
        required_height += 1; // Name label
        required_height += 1; // Name input
        required_height += 1; // Muscles label
        required_height += 1; // Muscles input
        required_height += 1; // Type label
        required_height += 1; // Type options
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error message line
        }
        // Add a little extra vertical padding if desired
        // required_height += 1;

        // --- Use centered_rect_fixed ---
        let fixed_width = 60; // Keep a fixed width (adjust as needed)
        let area = centered_rect_fixed(fixed_width, required_height, f.size());

        f.render_widget(Clear, area); // Clear the background
        f.render_widget(block, area); // Render the block border/title

        // Define the inner area *after* the block border/padding
        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        // --- Layout Constraints ---
        // Define constraints based on the required elements. The Min(0) handles extra space if any.
        let mut constraints = vec![
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Muscles label
            Constraint::Length(1), // Muscles input
            Constraint::Length(1), // Type label
            Constraint::Length(1), // Type options
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1)); // Error Message
        }
        constraints.push(Constraint::Min(0)); // Remainder (handles any extra space from fixed height)

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints) // Use the dynamically built constraints
            .split(inner_area);

        // --- Render Widgets ---
        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };
        let base_button_style = Style::default().fg(Color::White);

        // Row 1: Name
        f.render_widget(Paragraph::new("Name:"), chunks[0]);
        let name_style = if *focused_field == AddExerciseField::Name {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(name_input.as_str()).style(name_style),
            chunks[1].inner(&input_margin),
        );

        // Row 2: Muscles
        f.render_widget(Paragraph::new("Muscles (comma-separated):"), chunks[2]);
        let muscles_style = if *focused_field == AddExerciseField::Muscles {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(muscles_input.as_str()).style(muscles_style),
            chunks[3].inner(&input_margin),
        );

        // Row 3: Type
        f.render_widget(Paragraph::new("Type:"), chunks[4]);

        let type_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[5]); // Types in chunk 5

        // Render Type Options (same as before)
        let res_text = " Resistance ";
        let res_style = if *selected_type == ExerciseType::Resistance {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let res_para = Paragraph::new(res_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeResistance {
                res_style.reversed()
            } else {
                res_style
            });
        f.render_widget(res_para, type_layout[0]);

        let card_text = " Cardio ";
        let card_style = if *selected_type == ExerciseType::Cardio {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let card_para = Paragraph::new(card_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeCardio {
                card_style.reversed()
            } else {
                card_style
            });
        f.render_widget(card_para, type_layout[1]);

        let bw_text = " BodyWeight ";
        let bw_style = if *selected_type == ExerciseType::BodyWeight {
            base_button_style.bg(Color::DarkGray)
        } else {
            base_button_style
        };
        let bw_para = Paragraph::new(bw_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::TypeBodyweight {
                bw_style.reversed()
            } else {
                bw_style
            });
        f.render_widget(bw_para, type_layout[2]);

        // Row 4: Buttons (adjust chunk index based on error message presence)
        let button_chunk_index = if error_message.is_some() { 8 } else { 7 }; // Spacer is before buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);

        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddExerciseField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 5: Error Message (if present)
        if let Some(err) = error_message {
            // Error message is always the second to last chunk before Min(0)
            let error_chunk_index = chunks.len() - 2;
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // --- Cursor Positioning --- (remains the same logic)
        match focused_field {
            AddExerciseField::Name => {
                let cursor_x = (chunks[1].x + 1 + name_input.chars().count() as u16)
                    .min(chunks[1].right().saturating_sub(1));
                f.set_cursor(cursor_x, chunks[1].y);
            }
            AddExerciseField::Muscles => {
                let cursor_x = (chunks[3].x + 1 + muscles_input.chars().count() as u16)
                    .min(chunks[3].right().saturating_sub(1));
                f.set_cursor(cursor_x, chunks[3].y);
            }
            _ => {} // No cursor for type options or buttons
        }
    }
}

fn render_edit_workout_modal(f: &mut Frame, app: &App) {
    // This is almost identical to render_add_workout_modal, but with a different title,
    // a read-only exercise field, and no suggestions.
    if let ActiveModal::EditWorkout {
        // workout_id is not displayed directly
        exercise_name, // Display this
        sets_input,
        reps_input,
        weight_input,
        duration_input,
        distance_input,
        notes_input,
        focused_field,
        error_message,
        resolved_exercise,
        ..
        // No suggestion fields needed here
    } = &app.active_modal
    {
        let block = Block::default()
            .title(format!("Edit Workout Entry ({})", exercise_name)) // Use exercise name in title
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        // Calculate required height (similar to Add modal, but no exercise input focus/suggestions)
        let mut required_height = 2; // Borders/Padding
        required_height += 1; // Exercise display (read-only)
                              // required_height += 1; // No exercise input row
        required_height += 1; // Sets/Reps labels
        required_height += 1; // Sets/Reps inputs
        required_height += 1; // Weight/Duration labels
        required_height += 1; // Weight/Duration inputs
        required_height += 1; // Distance label
        required_height += 1; // Distance input
        required_height += 1; // Notes label
        required_height += 3; // Notes input (multi-line)
        required_height += 1; // Spacer
        required_height += 1; // Buttons row
        if error_message.is_some() {
            required_height += 1; // Error Message
        }

        let fixed_width = 80;
        let area = centered_rect_fixed(fixed_width, required_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let mut constraints = vec![
            Constraint::Length(1), // Exercise display
            // Constraint::Length(1), // No exercise input
            Constraint::Length(1), // Sets/Reps labels
            Constraint::Length(1), // Sets/Reps inputs
            Constraint::Length(1), // Weight/Duration labels
            Constraint::Length(1), // Weight/Duration inputs
            Constraint::Length(1), // Distance label
            Constraint::Length(1), // Distance input
            Constraint::Length(1), // Notes label
            Constraint::Length(3), // Notes input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Buttons row
        ];
        if error_message.is_some() {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Min(0));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };

        // Row 0: Exercise Display (read-only)
        f.render_widget(
            Paragraph::new(format!("Exercise: {}", exercise_name))
                .style(Style::default().fg(Color::DarkGray)), // Style as read-only
            chunks[0],
        );

        // Row 1: Sets/Reps Labels & Inputs (chunk indices shift by -1 compared to Add modal)
        let sets_reps_label_chunk_index = 1;
        let sets_reps_inputs_chunk_index = 2;
        let weight_dur_label_chunk_index = 3;
        let weight_dur_inputs_chunk_index = 4;
        let distance_label_chunk_index = 5;
        let distance_input_chunk_index = 6;
        let notes_label_chunk_index = 7;
        let notes_chunk_index = 8;
        let button_chunk_index = 10; // After spacer at 9
        let error_chunk_index = if error_message.is_some() { 11 } else { 10 }; // After buttons

        let sets_reps_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_label_chunk_index]);
        f.render_widget(Paragraph::new("Sets:"), sets_reps_layout[0]);
        f.render_widget(Paragraph::new("Reps:"), sets_reps_layout[1]);
        let sets_reps_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[sets_reps_inputs_chunk_index]);
        let sets_style = if *focused_field == AddWorkoutField::Sets {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(sets_input.as_str()).style(sets_style),
            sets_reps_inputs[0].inner(&input_margin),
        );
        let reps_style = if *focused_field == AddWorkoutField::Reps {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(reps_input.as_str()).style(reps_style),
            sets_reps_inputs[1].inner(&input_margin),
        );

        // Row 2: Weight/Duration Labels & Inputs
        let weight_unit = match app.service.config.units {
            Units::Metric => "kg",
            Units::Imperial => "lbs",
        };
        let weight_label_text = if resolved_exercise
            .as_ref()
            .map_or(false, |def| def.type_ == ExerciseType::BodyWeight)
        {
            format!("Added Weight ({}):", weight_unit)
        } else {
            format!("Weight ({}):", weight_unit)
        };
        let weight_dur_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_label_chunk_index]);
        f.render_widget(Paragraph::new(weight_label_text), weight_dur_layout[0]);
        f.render_widget(Paragraph::new("Duration (min):"), weight_dur_layout[1]);
        let weight_dur_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[weight_dur_inputs_chunk_index]);
        let weight_style = if *focused_field == AddWorkoutField::Weight {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            weight_dur_inputs[0].inner(&input_margin),
        );
        let dur_style = if *focused_field == AddWorkoutField::Duration {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(duration_input.as_str()).style(dur_style),
            weight_dur_inputs[1].inner(&input_margin),
        );

        // Row 3: Distance Label & Input
        let dist_unit = match app.service.config.units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        f.render_widget(
            Paragraph::new(format!("Distance ({}):", dist_unit)),
            chunks[distance_label_chunk_index],
        );
        let dist_style = if *focused_field == AddWorkoutField::Distance {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(distance_input.as_str()).style(dist_style),
            chunks[distance_input_chunk_index].inner(&input_margin),
        );

        // Row 4: Notes Label & Input
        f.render_widget(Paragraph::new("Notes:"), chunks[notes_label_chunk_index]);
        let notes_style = if *focused_field == AddWorkoutField::Notes {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(notes_input.as_str())
                .wrap(Wrap { trim: false })
                .style(notes_style)
                .block(Block::default().borders(Borders::LEFT)),
            chunks[notes_chunk_index].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );

        // Row 5: Buttons
        let base_button_style = Style::default().fg(Color::White);
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_chunk_index]);
        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Confirm {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(ok_button, button_layout[0]);
        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == AddWorkoutField::Cancel {
                base_button_style.reversed()
            } else {
                base_button_style
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Row 6: Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[error_chunk_index],
            );
        }

        // Cursor Positioning (Copied from Add modal, excluding Exercise/Suggestions)
        match focused_field {
            AddWorkoutField::Sets => f.set_cursor(
                sets_reps_inputs[0].x + 1 + sets_input.chars().count() as u16,
                sets_reps_inputs[0].y,
            ),
            AddWorkoutField::Reps => f.set_cursor(
                sets_reps_inputs[1].x + 1 + reps_input.chars().count() as u16,
                sets_reps_inputs[1].y,
            ),
            AddWorkoutField::Weight => f.set_cursor(
                weight_dur_inputs[0].x + 1 + weight_input.chars().count() as u16,
                weight_dur_inputs[0].y,
            ),
            AddWorkoutField::Duration => f.set_cursor(
                weight_dur_inputs[1].x + 1 + duration_input.chars().count() as u16,
                weight_dur_inputs[1].y,
            ),
            AddWorkoutField::Distance => f.set_cursor(
                chunks[distance_input_chunk_index].x + 1 + distance_input.chars().count() as u16,
                chunks[distance_input_chunk_index].y,
            ),
            AddWorkoutField::Notes => {
                let lines: Vec<&str> = notes_input.lines().collect();
                let last_line = lines.last().unwrap_or(&"");
                let notes_area = chunks[notes_chunk_index].inner(&Margin {
                    vertical: 0,
                    horizontal: 1,
                });
                let cursor_y = notes_area.y + lines.len().saturating_sub(1) as u16;
                let cursor_x = notes_area.x + last_line.chars().count() as u16;
                f.set_cursor(
                    cursor_x.min(notes_area.right() - 1),
                    cursor_y.min(notes_area.bottom() - 1),
                );
            }
            _ => {} // No cursor for buttons or read-only fields
        }
    }
}

// NEW: Render Confirmation Modal
fn render_confirmation_modal(f: &mut Frame, app: &App) {
    if let ActiveModal::ConfirmDeleteWorkout {
        exercise_name,
        set_index,
        ..
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)); // Make it stand out

        let question = format!("Delete set {} of {}?", set_index, exercise_name);
        let options = "[Y]es / [N]o (Esc)";

        // Calculate text width for centering
        let question_width = question.len() as u16;
        let options_width = options.len() as u16;
        let text_width = question_width.max(options_width);
        let modal_width = text_width + 4; // Add padding
        let modal_height = 5; // Fixed height: border + question + options + border

        let area = centered_rect_fixed(modal_width, modal_height, f.size());
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Question
                Constraint::Length(1), // Options
            ])
            .split(inner_area);

        f.render_widget(
            Paragraph::new(question).alignment(ratatui::layout::Alignment::Center),
            chunks[0],
        );
        f.render_widget(
            Paragraph::new(options).alignment(ratatui::layout::Alignment::Center),
            chunks[1],
        );

        // No cursor needed for this simple modal
    }
}
