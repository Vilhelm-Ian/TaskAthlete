// task-athlete-tui/src/ui/modals.rs
use crate::{
    app::{
        state::{ActiveModal, AddWorkoutField, LogBodyweightField, SetTargetWeightField},
        App,
    }, // Use App from crate::app
    ui::layout::centered_rect, // Use centered_rect from layout
                               // ui::layout::centered_rect_fixed,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use task_athlete_lib::{ExerciseType, Units};

pub fn render_modal(f: &mut Frame, app: &App) {
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f), // Don't need app state for help text
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
        ActiveModal::AddWorkout { .. } => render_add_workout_modal(f, app),
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
        resolved_exercise, // Get resolved exercise info
    } = &app.active_modal
    {
        let block = Block::default()
            .title("Add New Workout Entry")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());
        // Increased height to accommodate more fields
        let area = centered_rect(80, 40, f.size()); // Adjust size as needed
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Exercise label
                Constraint::Length(1), // Exercise input
                Constraint::Length(1), // Sets/Reps labels
                Constraint::Length(1), // Sets/Reps inputs
                Constraint::Length(1), // Weight/Duration labels
                Constraint::Length(1), // Weight/Duration inputs
                Constraint::Length(1), // Distance label
                Constraint::Length(1), // Distance input
                Constraint::Length(1), // Notes label
                Constraint::Length(3), // Notes input (multi-line)
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Buttons row
                Constraint::Length(1), // Error Message
                Constraint::Min(0),    // Remainder
            ])
            .split(inner_area);

        // Input Styling
        let base_input_style = Style::default().fg(Color::White);
        let input_margin = Margin {
            vertical: 0,
            horizontal: 1,
        };

        // --- Row 1: Exercise ---
        f.render_widget(Paragraph::new("Exercise Name/Alias:"), chunks[0]);
        let ex_style = if *focused_field == AddWorkoutField::Exercise {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(exercise_input.as_str()).style(ex_style),
            chunks[1].inner(&input_margin),
        );

        // --- Row 2: Sets/Reps ---
        let sets_reps_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);
        f.render_widget(Paragraph::new("Sets:"), sets_reps_layout[0]);
        f.render_widget(Paragraph::new("Reps:"), sets_reps_layout[1]);

        let sets_reps_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[3]);
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

        // --- Row 3: Weight/Duration ---
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
            .split(chunks[4]);
        f.render_widget(Paragraph::new(weight_label_text), weight_dur_layout[0]);
        f.render_widget(Paragraph::new("Duration (min):"), weight_dur_layout[1]);

        let weight_dur_inputs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[5]);
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

        // --- Row 4: Distance ---
        let dist_unit = match app.service.config.units {
            Units::Metric => "km",
            Units::Imperial => "mi",
        };
        f.render_widget(
            Paragraph::new(format!("Distance ({}):", dist_unit)),
            chunks[6],
        );
        let dist_style = if *focused_field == AddWorkoutField::Distance {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(distance_input.as_str()).style(dist_style),
            chunks[7].inner(&input_margin),
        );

        // --- Row 5: Notes ---
        f.render_widget(Paragraph::new("Notes:"), chunks[8]);
        let notes_style = if *focused_field == AddWorkoutField::Notes {
            base_input_style.reversed()
        } else {
            base_input_style
        };
        f.render_widget(
            Paragraph::new(notes_input.as_str())
                .wrap(Wrap { trim: false }) // Allow wrapping
                .style(notes_style)
                .block(Block::default().borders(Borders::LEFT)), // Indicate input area
            chunks[9].inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
        );

        // --- Row 6: Buttons ---
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[11]); // Buttons in chunk 11

        let base_button_style = Style::default().fg(Color::White);
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

        // --- Row 7: Error Message ---
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[12], // Error in chunk 12
            );
        }

        // --- Cursor Positioning ---
        match focused_field {
            AddWorkoutField::Exercise => f.set_cursor(
                chunks[1].x + 1 + exercise_input.chars().count() as u16,
                chunks[1].y,
            ),
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
                chunks[7].x + 1 + distance_input.chars().count() as u16,
                chunks[7].y,
            ),
            AddWorkoutField::Notes => {
                // Basic cursor positioning for notes (end of content)
                let lines: Vec<&str> = notes_input.lines().collect();
                let last_line = lines.last().unwrap_or(&"");
                let cursor_y = chunks[9].y + lines.len().saturating_sub(1) as u16;
                let cursor_x = chunks[9].x + 1 + last_line.chars().count() as u16; // +1 for the border/padding
                f.set_cursor(
                    cursor_x.min(chunks[9].right() - 1),
                    cursor_y.min(chunks[9].bottom() - 1),
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
