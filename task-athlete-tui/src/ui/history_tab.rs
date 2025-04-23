// src/ui/history_tab.rs
use crate::app::{
    data::{format_date_with_ordinal, format_set_line}, // Make sure helpers are imported
    state::{App, HistoryFocus},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;
use task_athlete_lib::{Units, Workout}; // Make sure Units is imported

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title("Workout History")
        .border_style(if app.history_focus == HistoryFocus::DayList {
            Style::default().fg(Color::Yellow) // Highlight outer block if focused
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    if app.history_data.is_empty() {
        f.render_widget(
            Paragraph::new("No history data found.").alignment(ratatui::layout::Alignment::Center),
            inner_area,
        );
        return;
    }

    let data_len = app.history_data.len();
    let current_selection = match app.history_list_state.selected() {
        Some(i) if i < data_len => i,
        Some(_) => data_len.saturating_sub(1),
        None if data_len > 0 => 0,
        None => {
            f.render_widget(Paragraph::new("No selection."), inner_area);
            return;
        }
    };

    // Ensure selection is set if currently None
    if app.history_list_state.selected().is_none() && data_len > 0 {
        app.history_list_state.select(Some(0));
    }

    // --- Pre-calculate Heights and Content for Visible Items ---

    let available_height = inner_area.height;
    let mut constraints = Vec::new();
    // Store tuple: (original_index, title, lines, is_selected, calculated_height)
    let mut visible_items_data = Vec::new();
    let mut total_calculated_height = 0;

    // 1. Render selected item and items *after* it
    for index in current_selection..data_len {
        if let Some((date, workouts)) = app.history_data.get(index) {
            let is_selected = index == current_selection;
            let title = format_date_with_ordinal(*date);
            let content_lines = format_day_workout_lines(workouts, app.service.config.units);
            // Height = lines + block top border + block bottom border
            let required_height = (content_lines.len() as u16).saturating_add(2);

            if total_calculated_height + required_height <= available_height {
                total_calculated_height += required_height;
                constraints.push(Constraint::Length(required_height));
                visible_items_data.push((
                    index,
                    title,
                    content_lines,
                    is_selected,
                    required_height,
                ));
            } else {
                break; // Not enough space for this item
            }
        }
    }

    // 2. If space remaining, render items *before* the selected one (in reverse order of processing)
    if total_calculated_height < available_height && current_selection > 0 {
        for index in (0..current_selection).rev() {
            // Iterate backwards from selection - 1
            if let Some((date, workouts)) = app.history_data.get(index) {
                let is_selected = false; // These are definitely not selected
                let title = format_date_with_ordinal(*date);
                let content_lines = format_day_workout_lines(workouts, app.service.config.units);
                let required_height = (content_lines.len() as u16).saturating_add(2);

                if total_calculated_height + required_height <= available_height {
                    total_calculated_height += required_height;
                    // Insert at the beginning to maintain visual order
                    constraints.insert(0, Constraint::Length(required_height));
                    visible_items_data.insert(
                        0,
                        (index, title, content_lines, is_selected, required_height),
                    );
                } else {
                    break; // Not enough space for this item
                }
            }
        }
    }

    // 3. Add filler constraint if needed (though maybe not necessary with exact lengths)
    // if total_calculated_height < available_height {
    //     constraints.push(Constraint::Min(0));
    // }

    // --- Create Layout and Render ---

    // Check if we actually have items to render before creating layout
    if constraints.is_empty() {
        // This might happen if the selected item itself is too tall
        // Render just the selected item clipped? Or show an error?
        // Let's try rendering just the selected one clipped for now.
        if let Some((date, workouts)) = app.history_data.get(current_selection) {
            let title = format_date_with_ordinal(*date);
            let content_lines = format_day_workout_lines(workouts, app.service.config.units);
            let day_block = Block::default()
                .title(title.as_str())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)); // Selected is always yellow here
            let paragraph = Paragraph::new(Text::from(content_lines)).block(day_block);
            f.render_widget(paragraph, inner_area); // Render directly into inner_area
        } else {
            f.render_widget(
                Paragraph::new("Error displaying selected item."),
                inner_area,
            );
        }
        return; // Exit render function
    }

    let item_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints) // Use calculated exact lengths
        .split(inner_area);

    // Render the calculated visible items
    for (i, (_original_index, title, content_lines, is_selected, _height)) in
        visible_items_data.iter().enumerate()
    {
        // Check if chunk exists for safety, though layout should match visible_items_data length
        if let Some(chunk) = item_chunks.get(i) {
            let border_style = if *is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let day_block = Block::default()
                .title(title.as_str())
                .borders(Borders::ALL)
                .border_style(border_style);

            let paragraph = Paragraph::new(Text::from(content_lines.clone())).block(day_block);
            f.render_widget(paragraph, *chunk);
        }
    }
}

// format_day_workout_lines function remains the same as the corrected version from the previous step
fn format_day_workout_lines(workouts: &[Workout], units: Units) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    // Don't add initial spacer here, add it before calling this function if needed,
    // or let the Paragraph widget handle padding within its block.

    // 1. Group workouts by exercise name
    let mut workouts_by_exercise: HashMap<String, Vec<&Workout>> = HashMap::new();
    for workout in workouts {
        workouts_by_exercise
            .entry(workout.exercise_name.clone())
            .or_default()
            .push(workout);
    }

    // 2. Sort exercise names for consistent order
    let mut sorted_exercise_names: Vec<String> = workouts_by_exercise.keys().cloned().collect();
    sorted_exercise_names.sort_unstable(); // Sort alphabetically

    // 3. Process each exercise group
    for exercise_name in sorted_exercise_names {
        // Add exercise name title
        lines.push(Line::from(Span::styled(
            exercise_name.clone(),
            Style::default().add_modifier(Modifier::BOLD), // Use Modifier::BOLD
        )));

        if let Some(sets) = workouts_by_exercise.get(&exercise_name) {
            // Temporary storage for lines related to THIS exercise's sets
            let mut current_exercise_set_lines: Vec<Line<'static>> = Vec::new();

            if !sets.is_empty() {
                // Initialize tracking with the first set
                let mut previous_set_string = format_set_line(sets[0], units);
                let mut repetition_count = 1;

                // Iterate from the second set onwards (index 1)
                for i in 1..sets.len() {
                    let current_formatted_set = format_set_line(sets[i], units);

                    // Check if the current set is identical to the previous one
                    if current_formatted_set == previous_set_string
                        && !previous_set_string.is_empty()
                    {
                        repetition_count += 1; // Increment count for consecutive identical sets
                    } else {
                        // The set has changed, or we encountered an empty formatted string.
                        // Format and add the previous group/set to the temporary list.
                        let line_to_add = if repetition_count > 1 {
                            format!("  {}x {}", repetition_count, previous_set_string)
                        // Use 'N x ...' format
                        } else {
                            format!("  {}", previous_set_string) // Single occurrence, no '1 x' prefix
                        };
                        // Only add if the string wasn't empty to begin with
                        if !previous_set_string.is_empty() {
                            current_exercise_set_lines.push(Line::from(line_to_add).to_owned());
                        }

                        // Reset tracking for the new set
                        previous_set_string = current_formatted_set;
                        repetition_count = 1; // Reset count to 1 for the new distinct set
                    }
                }

                // After the loop, add the very last group/set that was being tracked
                if !previous_set_string.is_empty() {
                    // Ensure we don't add an empty line if the last set was invalid/empty
                    let last_line_to_add = if repetition_count > 1 {
                        format!("  {}x {}", repetition_count, previous_set_string)
                    } else {
                        format!("  {}", previous_set_string)
                    };
                    current_exercise_set_lines.push(Line::from(last_line_to_add).to_owned());
                }
            }

            // 4. Reverse the order of processed sets for this exercise
            current_exercise_set_lines.reverse();

            // 5. Extend the main 'lines' Vec with the reversed sets for this exercise
            lines.extend(current_exercise_set_lines);
        }
        // Add a spacer line after each exercise's sets for readability
        lines.push(Line::from(""));
    }

    // Remove the last spacer line if it exists
    if !lines.is_empty() && lines.last().map_or(false, |l| l.width() == 0) {
        lines.pop();
    }

    lines // Return the formatted lines for the entire day
}
