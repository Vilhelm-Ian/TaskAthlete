// src/ui/graphs_tab.rs
use crate::app::{
    data::graph_type_to_string, // Import helper
    state::{App, GraphsFocus},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::Span,
    widgets::{
        Axis,
        Block,
        Borders,
        Chart,
        Dataset,
        GraphType as ChartGraphType,
        List,
        ListItem,
        Paragraph,
    },
    Frame,
};
use task_athlete_lib::{GraphType as LibGraphType, Units}; // Import Units

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Exercise List
            Constraint::Percentage(25), // Graph Type List
            Constraint::Percentage(45), // Graph Display
        ])
        .split(area);

    render_graph_exercise_list(f, app, chunks[0]);
    render_graph_type_list(f, app, chunks[1]);
    render_graph_display(f, app, chunks[2]);
}

fn render_graph_exercise_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .graph_exercises_all // Use the dedicated list for this tab
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title("Select Exercise")
        .border_style(if let GraphsFocus::ExerciseList = app.graph_focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.graph_exercise_list_state);
}

fn render_graph_type_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .graph_types_available
        .iter()
        .map(|graph_type| ListItem::new(graph_type_to_string(*graph_type))) // Use helper
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title("Select Graph Type")
        .border_style(if let GraphsFocus::GraphTypeList = app.graph_focus {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.graph_type_list_state);
}

fn render_graph_display(f: &mut Frame, app: &App, area: Rect) {
    // Determine title based on selection
    let graph_pane_title = match (
        app.graph_selected_exercise.as_ref(),
        app.graph_selected_type,
    ) {
        (Some(ex_name), Some(graph_type)) => {
            format!("{} - {}", ex_name, graph_type_to_string(graph_type))
        }
        (Some(ex_name), None) => format!("{} - Select Graph Type", ex_name),
        _ => "Select Exercise and Graph Type".to_string(),
    };

    let graph_block = Block::default()
        .borders(Borders::ALL)
        .title(graph_pane_title)
        .border_style(Style::default().fg(Color::DarkGray)); // No specific focus for graph display pane itself

    if app.graph_data_points.is_empty()
        || app.graph_selected_exercise.is_none()
        || app.graph_selected_type.is_none()
    {
        // Render placeholder if no data or selection
        f.render_widget(graph_block, area);
        // Optionally add a paragraph inside saying "No data..."
        let center_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Min(1),
                Constraint::Percentage(50),
            ])
            .split(area)[1];
        let center_area_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Min(10),
                Constraint::Percentage(20),
            ])
            .split(center_area)[1];
        f.render_widget(
            Paragraph::new("No data to display.").alignment(ratatui::layout::Alignment::Center),
            center_area_h,
        );

        return;
    }

    // --- Prepare Chart ---
    // Fix: Convert Option<&String> to Option<&str> before unwrapping or providing default &str
    let dataset_name = app
        .graph_selected_exercise
        .as_deref() // Get Option<&str>
        .unwrap_or(""); // Default to empty &str if None

    let datasets = vec![Dataset::default()
        .name(dataset_name) // Use the &str
        .marker(symbols::Marker::Dot)
        .graph_type(ChartGraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&app.graph_data_points)];

    // Determine axis titles based on graph type and units
    let (y_title, x_title) =
        get_axis_titles(app.graph_selected_type.unwrap(), &app.service.config.units);

    // Create labels for Y axis (simplistic for now)
    let y_labels: Vec<Span> = if app.graph_y_bounds[0] < app.graph_y_bounds[1] {
        let min_label = app.graph_y_bounds[0].ceil();
        let max_label = app.graph_y_bounds[1].floor();
        let range = max_label - min_label;
        let step = (range / 5.0).max(1.0); // Aim for ~5 labels

        (0..=5)
            .map(|i| {
                let val = min_label + step * i as f64;
                // Format based on magnitude? Simple format for now.
                if val.fract() == 0.0 {
                    format!("{:.0}", val)
                } else if range < 10.0 {
                    // More precision for small ranges
                    format!("{:.1}", val)
                } else {
                    format!("{:.0}", val)
                }
            })
            .map(Span::raw)
            .collect()
    } else {
        vec![Span::raw("0")] // Default if bounds are bad
    };

    let chart = Chart::new(datasets)
        .block(graph_block) // Use the block defined earlier
        .x_axis(
            Axis::default()
                .title(x_title.italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(app.graph_x_bounds)
                // Add labels if needed (e.g., based on date range), skip for now
                .labels(vec![]),
        )
        .y_axis(
            Axis::default()
                .title(y_title.italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(app.graph_y_bounds)
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

// Helper to determine axis titles
fn get_axis_titles(graph_type: LibGraphType, units: &Units) -> (String, String) {
    let x_title = "Days Since First Workout".to_string();
    let y_title = match graph_type {
        LibGraphType::Estimated1RM | LibGraphType::MaxWeight => {
            // Corrected: Access config units via app.service.config
            format!(
                "Weight ({})",
                if units == &Units::Metric { "kg" } else { "lbs" }
            )
        }
        LibGraphType::MaxReps | LibGraphType::WorkoutReps => "Reps".to_string(),
        LibGraphType::WorkoutVolume => {
            // Corrected: Access config units via app.service.config
            format!(
                "Volume ({})",
                if units == &Units::Metric { "kg" } else { "lbs" }
            ) // Volume units depends on weight unit
        }
        LibGraphType::WorkoutDuration => "Duration (min)".to_string(),
        LibGraphType::WorkoutDistance => {
            // Corrected: Access config units via app.service.config
            format!(
                "Distance ({})",
                if units == &Units::Metric { "km" } else { "mi" }
            )
        }
    };
    (y_title, x_title)
}
