// task-athlete-tui/src/ui.rs
use crate::app::{ActiveModal, ActiveTab, App, BodyweightFocus, LogFocus};
use chrono::{Duration, NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        block::{Position, Title}, // Import Title
        Axis,
        Block,
        Borders,
        Cell,
        Chart,
        Clear,
        Dataset,
        GraphType,
        LegendPosition,
        List,
        ListItem,
        Paragraph,
        Row,
        Scrollbar,
        ScrollbarOrientation,
        ScrollbarState,
        Table,
        Tabs,
        Wrap,
    },
    Frame,
};
use task_athlete_lib::Units; // Import Units

// Main UI rendering function
pub fn render_ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Create main layout: Tabs on top, content below, status bar at bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status Bar
        ])
        .split(size);

    render_tabs(f, app, main_chunks[0]);
    render_main_content(f, app, main_chunks[1]);
    render_status_bar(f, app, main_chunks[2]);

    // Render modal last if active
    if app.active_modal != ActiveModal::None {
        render_modal(f, app);
    }
}

// Render the top tabs
fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ["Log (F1)", "History (F2)", "Graphs (F3)", "Bodyweight (F4)"]
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::Gray))))
        .collect();

    let selected_tab_index = match app.active_tab {
        ActiveTab::Log => 0,
        ActiveTab::History => 1,
        ActiveTab::Graphs => 2,
        ActiveTab::Bodyweight => 3,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(selected_tab_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

// Render the content area based on the active tab
fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let content_block = Block::default().borders(Borders::NONE); // Add border if desired
    f.render_widget(content_block, area);
    let content_area = area.inner(&ratatui::layout::Margin {
        vertical: 0,
        horizontal: 0,
    }); // Adjust margin if block has borders

    match app.active_tab {
        ActiveTab::Log => render_log_tab(f, app, content_area),
        ActiveTab::History => render_placeholder(f, "History Tab", content_area), // Placeholder
        ActiveTab::Graphs => render_placeholder(f, "Graphs Tab", content_area),   // Placeholder
        ActiveTab::Bodyweight => render_bodyweight_tab(f, app, content_area),
    }
}

// Placeholder for unimplemented tabs
fn render_placeholder(f: &mut Frame, title: &str, area: Rect) {
    let placeholder_text = Paragraph::new(format!("{} - Implementation Pending", title))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    f.render_widget(placeholder_text, area);
}

// --- Log Tab Rendering ---
fn render_log_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let today_str = Utc::now().date_naive();
    let date_header_str = if app.log_viewed_date == today_str {
        format!("--- Today ({}) ---", app.log_viewed_date.format("%Y-%m-%d"))
    } else if app.log_viewed_date == today_str - Duration::days(1) {
        format!(
            "--- Yesterday ({}) ---",
            app.log_viewed_date.format("%Y-%m-%d")
        )
    } else {
        format!("--- {} ---", app.log_viewed_date.format("%Y-%m-%d"))
    };

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let date_header = Paragraph::new(date_header_str).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(date_header, outer_chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer_chunks[1]);

    render_log_exercise_list(f, app, chunks[0]);
    render_log_set_list(f, app, chunks[1]);
}

fn render_log_exercise_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .log_exercises_today
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title("Exercises Logged")
        .border_style(if app.log_focus == LogFocus::ExerciseList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray) // Change background on selection
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Use stateful widget
    f.render_stateful_widget(list, area, &mut app.log_exercise_list_state);
}

fn render_log_set_list(f: &mut Frame, app: &mut App, area: Rect) {
    let selected_exercise_name = app
        .log_exercise_list_state
        .selected()
        .and_then(|i| app.log_exercises_today.get(i));

    let title = match selected_exercise_name {
        Some(name) => format!("Sets for: {}", name),
        None => "Select an Exercise".to_string(),
    };

    let table_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if app.log_focus == LogFocus::SetList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let header_cells = ["Set", "Reps", "Weight", "Duration", "Distance", "Notes"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let dist_unit = match app.service.config.units {
        Units::Metric => "km",
        Units::Imperial => "mi",
    };

    let rows = app
        .log_sets_for_selected_exercise
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let weight_str = w
                .weight
                .map_or("-".to_string(), |v| format!("{:.1} {}", v, weight_unit));
            let dist_val = match app.service.config.units {
                Units::Metric => w.distance,
                Units::Imperial => w.distance.map(|km| km * 0.621371), // Convert km to miles
            };
            let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1} {}", v, dist_unit));

            Row::new(vec![
                Cell::from(format!("{}", i + 1)), // Set number (or entry number)
                Cell::from(w.reps.map_or("-".to_string(), |v| v.to_string())),
                Cell::from(weight_str),
                Cell::from(
                    w.duration_minutes
                        .map_or("-".to_string(), |v| format!("{} min", v)),
                ),
                Cell::from(dist_str),
                Cell::from(w.notes.clone().unwrap_or_else(|| "-".to_string())),
            ])
        });

    // Calculate widths dynamically? Or fixed for now? Fixed for simplicity.
    let widths = [
        Constraint::Length(5),
        Constraint::Length(6),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Min(10), // Notes can expand
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.log_set_table_state);
}

// --- Bodyweight Tab Rendering ---
fn render_bodyweight_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Graph
            Constraint::Percentage(50), // Bottom area
        ])
        .split(area);

    render_bodyweight_graph(f, app, chunks[0]);
    render_bodyweight_bottom(f, app, chunks[1]);
}

fn render_bodyweight_graph(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let target_data;

    let mut datasets = vec![];

    // Bodyweight Data
    let data_points: Vec<(f64, f64)> = app
        .bw_graph_data
        .iter()
        .map(|(x, y)| {
            let display_weight = match app.service.config.units {
                Units::Metric => *y,
                Units::Imperial => *y * 2.20462, // Convert kg to lbs if necessary
            };
            (*x, display_weight)
        })
        .collect();

    datasets.push(
        Dataset::default()
            .name("Bodyweight")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points), // Pass reference
    );

    // Target Bodyweight Line
    if let Some(target_raw) = app.bw_target {
        let target_display = match app.service.config.units {
            Units::Metric => target_raw,
            Units::Imperial => target_raw * 2.20462,
        };
        // Create a dataset for the horizontal line
        target_data = vec![
            (app.bw_graph_x_bounds[0], target_display), // Start point
            (app.bw_graph_x_bounds[1], target_display), // End point
        ];
        datasets.push(
            Dataset::default()
                .name("Target")
                .marker(symbols::Marker::Braille) // No markers needed for a line
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::ITALIC),
                )
                .data(&target_data),
        );
    }

    // Calculate display Y-bounds based on units
    let display_y_bounds = match app.service.config.units {
        Units::Metric => app.bw_graph_y_bounds,
        Units::Imperial => [
            app.bw_graph_y_bounds[0] * 2.20462,
            app.bw_graph_y_bounds[1] * 2.20462,
        ],
    };

    // Create the chart
    let range_label = match app.bw_graph_range_months {
        1 => "1M",
        3 => "3M",
        6 => "6M",
        12 => "1Y",
        _ => "All",
    };
    let chart_title = format!("Bodyweight Trend ({})", range_label);

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(chart_title))
        .x_axis(
            Axis::default()
                .title("Date".italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(app.bw_graph_x_bounds) // Use calculated bounds
                .labels(vec![]), // Disable numeric labels for time axis for now
        )
        .y_axis(
            Axis::default()
                .title(format!("Weight ({})", weight_unit).italic())
                .style(Style::default().fg(Color::Gray))
                .bounds(display_y_bounds) // Use unit-adjusted bounds
                .labels(
                    (display_y_bounds[0].floor() as usize..=display_y_bounds[1].floor() as usize)
                        .step_by(
                            ((display_y_bounds[1] - display_y_bounds[0]) / 5.0).max(1.0) as usize
                        ) // Adjust step
                        .map(|w| Span::from(format!("{:.1}", w)))
                        .collect(),
                ),
        )
        .legend_position(Some(LegendPosition::TopLeft)); // Add legend

    f.render_widget(chart, area);
}

fn render_bodyweight_bottom(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Status & Actions
            Constraint::Percentage(60), // History
        ])
        .split(area);

    render_bodyweight_status(f, app, chunks[0]);
    render_bodyweight_history(f, app, chunks[1]);
}

fn render_bodyweight_status(f: &mut Frame, app: &App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    let latest_weight_str = match app.bw_latest {
        Some(w) => {
            let display_w = match app.service.config.units {
                Units::Metric => w,
                Units::Imperial => w * 2.20462,
            };
            format!("{:.1} {}", display_w, weight_unit)
        }
        None => "N/A".to_string(),
    };

    let target_weight_str = match app.bw_target {
        Some(w) => {
            let display_w = match app.service.config.units {
                Units::Metric => w,
                Units::Imperial => w * 2.20462,
            };
            format!("{:.1} {}", display_w, weight_unit)
        }
        None => "Not Set".to_string(),
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Latest: ", Style::default().bold()),
            Span::raw(latest_weight_str),
        ]),
        Line::from(vec![
            Span::styled("Target: ", Style::default().bold()),
            Span::raw(target_weight_str),
        ]),
        Line::from(""), // Spacer
        Line::from(Span::styled(
            " [L]og New ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [T]arget Weight ",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            " [R]ange Cycle ",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status & Actions"),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_bodyweight_history(f: &mut Frame, app: &mut App, area: Rect) {
    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };

    let table_block = Block::default()
        .borders(Borders::ALL)
        .title("History")
        .border_style(if app.bw_focus == BodyweightFocus::History {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let weigth_cell = format!("Weight ({})", weight_unit);
    let header_cells = ["Date", &weigth_cell]
        .into_iter()
        .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.bw_history.iter().map(|(_, date, weight_kg)| {
        let display_weight = match app.service.config.units {
            Units::Metric => *weight_kg,
            Units::Imperial => *weight_kg * 2.20462, // Convert kg to lbs if necessary
        };
        Row::new(vec![
            Cell::from(date.format("%Y-%m-%d").to_string()),
            Cell::from(format!("{:.1}", display_weight)),
        ])
    });

    let widths = [Constraint::Length(12), Constraint::Min(10)];
    let table = Table::new(rows, widths)
        .header(header)
        .block(table_block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.bw_history_state);

    // Add scrollbar if needed (example)
    // let scrollbar = Scrollbar::default()
    //     .orientation(ScrollbarOrientation::VerticalRight)
    //     .begining_symbol(Some("↑"))
    //     .end_symbol(Some("↓"));
    // let mut scrollbar_state = ScrollbarState::new(app.bw_history.len()).position(app.bw_history_state.selected().unwrap_or(0)); // Position based on selection
    // f.render_stateful_widget(scrollbar, area.inner(&Margin{vertical: 1, horizontal: 0}), &mut scrollbar_state); // Render inside block margin
}

// --- Status Bar Rendering ---
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_text = match app.active_modal {
        ActiveModal::None => " [?] Help | [F1-F4] Tabs | [Q]uit ".to_string(),
        ActiveModal::Help => " [Esc] Close Help ".to_string(),
        // Add hints for other modals
    };

    let error_text = app.last_error.as_deref().unwrap_or("");

    // Create layout for status bar (left-aligned hints, right-aligned error)
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let status_paragraph =
        Paragraph::new(status_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status_paragraph, status_chunks[0]);

    let error_paragraph = Paragraph::new(error_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::Red))
        .alignment(ratatui::layout::Alignment::Right);
    f.render_widget(error_paragraph, status_chunks[1]);
}

// --- Modal Rendering ---
fn render_modal(f: &mut Frame, app: &App) {
    match app.active_modal {
        ActiveModal::Help => render_help_modal(f, app),
        // Handle other modals
        _ => {}
    }
}

fn render_help_modal(f: &mut Frame, _app: &App) {
    let block = Block::default()
        .title("Help (?)")
        .borders(Borders::ALL)
        .title_style(Style::new().bold())
        .border_style(Style::new().yellow());
    let area = centered_rect(60, 50, f.size()); // Helper function to create centered Rect
    f.render_widget(Clear, area); // Clear background
    f.render_widget(block, area);

    let help_text = vec![
        Line::from("--- Global ---"),
        Line::from(" Q: Quit"),
        Line::from(" ?: Show/Hide Help"),
        Line::from(" F1-F4: Switch Tabs"),
        Line::from(""),
        Line::from("--- Log Tab ---"),
        Line::from(" j/k / ↓/↑: Navigate lists"),
        Line::from(" Tab: Switch Focus (Exercises <=> Sets)"),
        Line::from(" h/l / ←/→: Change Viewed Date"),
        Line::from(" a: Add New Workout (TODO)"),
        Line::from(" l: Log New Set for Selected Exercise (TODO)"),
        Line::from(" e/Enter: Edit Selected Set (TODO)"),
        Line::from(" d/Delete: Delete Selected Set (TODO)"),
        Line::from(" g: Go to Graphs for Exercise (TODO)"),
        Line::from(""),
        Line::from("--- Bodyweight Tab ---"),
        Line::from(" j/k / ↓/↑: Navigate History Table"),
        Line::from(" Tab: Cycle Focus (TODO)"),
        Line::from(" l: Log New Bodyweight (TODO)"),
        Line::from(" t: Set Target Bodyweight (TODO)"),
        Line::from(" r: Cycle Graph Time Range"),
        Line::from(" d/Delete: Delete Selected History Entry"),
        Line::from(""),
        Line::from("--- Other Tabs (TODO) ---"),
        Line::from(" / or f: Activate Filter (History)"),
        Line::from(" Enter: Edit / Confirm Selection"),
        Line::from(" Esc: Exit Filter / Close Modal"),
        Line::from(""),
        Line::from(Span::styled(
            " Press Esc or ? to close ",
            Style::new().italic().yellow(),
        )),
    ];

    let paragraph = Paragraph::new(help_text)
        .wrap(Wrap { trim: false }) // Don't trim help text
        .block(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::new().on_dark_gray()),
        ); // Inner style for text area

    // Render paragraph inside the block's inner area
    f.render_widget(
        paragraph,
        area.inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}

/// Helper function to create a centered rectangle for modals
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
