// task-athlete-tui/src/ui.rs
use crate::app::{
    ActiveModal, ActiveTab, App, BodyweightFocus, LogBodyweightField, LogFocus,
    SetTargetWeightField,
}; // Add modal fields
use chrono::{Duration, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, LegendPosition, List,
        ListItem, Paragraph, Row, Table, Tabs, Wrap,
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

    let weight_unit = match app.service.config.units {
        Units::Metric => "kg",
        Units::Imperial => "lbs",
    };
    let dist_unit = match app.service.config.units {
        Units::Metric => "km",
        Units::Imperial => "mi",
    };
    let weight_cell = format!("Weight ({})", weight_unit);
    let distance_cell = format!("Distance ({})", dist_unit);
    let header_cells = [
        "Set",
        "Reps",
        &weight_cell,
        "Duration",
        &distance_cell,
        "Notes",
    ]
    .into_iter()
    .map(|h| Cell::from(h).style(Style::default().fg(Color::LightBlue)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app
        .log_sets_for_selected_exercise
        .iter()
        .enumerate()
        .map(|(i, w)| {
            // Format weight based on units
            let weight_display = match app.service.config.units {
                Units::Metric => w.weight,
                Units::Imperial => w.weight.map(|kg| kg * 2.20462),
            };
            let weight_str = weight_display.map_or("-".to_string(), |v| format!("{:.1}", v)); // Remove unit string, column header has it

            // Format distance based on units
            let dist_val = match app.service.config.units {
                Units::Metric => w.distance,
                Units::Imperial => w.distance.map(|km| km * 0.621_371), // Convert km to miles
            };
            let dist_str = dist_val.map_or("-".to_string(), |v| format!("{:.1}", v)); // Remove unit string

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
        Constraint::Length(5),  // Set
        Constraint::Length(6),  // Reps
        Constraint::Length(8),  // Weight
        Constraint::Length(10), // Duration
        Constraint::Length(10), // Distance
        Constraint::Min(10),    // Notes can expand
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
    let target_data; // Needs to live long enough

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
        // Create a dataset for the horizontal line, ensure bounds are valid
        if app.bw_graph_x_bounds[0] <= app.bw_graph_x_bounds[1] {
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(chart_title)
                .border_style(if app.bw_focus == BodyweightFocus::Graph {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
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
                    // Dynamically generate labels based on bounds
                    {
                        let min_label = display_y_bounds[0].ceil() as i32;
                        let max_label = display_y_bounds[1].floor() as i32;
                        let range = (max_label - min_label).max(1); // Avoid division by zero
                        let step = (range / 5).max(1); // Aim for ~5 labels

                        (min_label..=max_label)
                            .step_by(step as usize)
                            .map(|w| Span::from(format!("{:.0}", w))) // Use integer format for simplicity
                            .collect()
                    },
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

    // Get latest weight and date
    let (latest_weight_str, latest_date_str) = match app.bw_history.first() {
        Some((_, date, w)) => {
            let display_w = match app.service.config.units {
                Units::Metric => *w,
                Units::Imperial => *w * 2.20462,
            };
            (
                format!("{:.1} {}", display_w, weight_unit),
                format!("(on {})", date.format("%Y-%m-%d")),
            )
        }
        None => ("N/A".to_string(), "".to_string()),
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
            Span::styled(
                format!(" {}", latest_date_str),
                Style::default().fg(Color::DarkGray),
            ),
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
                .title("Status & Actions")
                .border_style(if app.bw_focus == BodyweightFocus::Actions {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
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

    let weight_cell_header = format!("Weight ({})", weight_unit); // Create String here
    let header_cells = ["Date", weight_cell_header.as_str()] // Use slice ref
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
        ActiveModal::None => match app.active_tab {
            ActiveTab::Log => "[Tab] Focus | [↑↓/jk] Nav | [←→/hl] Date | [a]dd | [l]og set | [e]dit | [d]elete | [g]raphs | [?] Help | [Q]uit ",
            ActiveTab::History => "[↑↓/jk] Nav | [/f] Filter | [e]dit | [d]elete | [?] Help | [Q]uit ",
            ActiveTab::Graphs => "[Tab] Focus | [↑↓/jk] Nav | [/] Filter Exercise | [Enter] Select | [?] Help | [Q]uit ",
            ActiveTab::Bodyweight => "[↑↓/jk] Nav Hist | [l]og | [t]arget | [r]ange | [?] Help | [Q]uit ",
        }.to_string(),
        ActiveModal::Help => " [Esc/Enter/?] Close Help ".to_string(),
        ActiveModal::LogBodyweight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
        ActiveModal::SetTargetWeight { .. } => " [Esc] Cancel | [Enter] Confirm | [Tab/↑↓] Navigate ".to_string(),
        // Add hints for other modals
    };

    let error_text = app.last_error.as_deref().unwrap_or("");

    // Create layout for status bar (left-aligned hints, right-aligned error)
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)]) // Adjust percentage
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
    match &app.active_modal {
        ActiveModal::Help => render_help_modal(f, app),
        ActiveModal::LogBodyweight { .. } => render_log_bodyweight_modal(f, app),
        ActiveModal::SetTargetWeight { .. } => render_set_target_weight_modal(f, app),
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
    let area = centered_rect(60, 70, f.size()); // Increase height slightly
    f.render_widget(Clear, area); // Clear background
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
        //Line::from(" d / Delete: Delete Selected History Entry (TODO)"), // User requested not to implement delete
        Line::from(""),
        Line::from(Span::styled(
            " Press Esc, ?, or Enter to close ",
            Style::new().italic().yellow(),
        )),
    ];

    let paragraph = Paragraph::new(help_text)
        .wrap(Wrap { trim: false }) // Don't trim help text
        .block(Block::default().borders(Borders::NONE));

    // Render paragraph inside the block's inner area
    f.render_widget(
        paragraph,
        area.inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}

// --- Specific Modal Renderers ---

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
        let area = centered_rect(50, 11, f.size()); // Adjusted height for content + error
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Weight label
                Constraint::Length(1), // Weight input
                Constraint::Length(1), // Date label
                Constraint::Length(1), // Date input
                Constraint::Length(1), // Buttons
                Constraint::Length(1), // Spacer for error
                Constraint::Length(1), // Error message
            ])
            .split(area.inner(&ratatui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            }));

        // Labels
        f.render_widget(
            Paragraph::new(format!("Weight ({}):", weight_unit)),
            chunks[0],
        );
        f.render_widget(Paragraph::new("Date (YYYY-MM-DD / today):"), chunks[2]);

        // Input Fields
        let weight_style = if *focused_field == LogBodyweightField::Weight {
            Style::default().reversed()
        } else {
            Style::default()
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            chunks[1],
        );

        let date_style = if *focused_field == LogBodyweightField::Date {
            Style::default().reversed()
        } else {
            Style::default()
        };
        f.render_widget(
            Paragraph::new(date_input.as_str()).style(date_style),
            chunks[3],
        );

        // Buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[4]);

        let ok_button = Paragraph::new(" OK ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Confirm {
                Style::default().reversed()
            } else {
                Style::default()
            });
        f.render_widget(ok_button, button_layout[0]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == LogBodyweightField::Cancel {
                Style::default().reversed()
            } else {
                Style::default()
            });
        f.render_widget(cancel_button, button_layout[1]);

        // Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[6],
            );
        }

        // Set cursor position (optional but helpful)
        match focused_field {
            LogBodyweightField::Weight => f.set_cursor(
                chunks[1].x + weight_input.chars().count() as u16,
                chunks[1].y,
            ),
            LogBodyweightField::Date => {
                f.set_cursor(chunks[3].x + date_input.chars().count() as u16, chunks[3].y)
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
        let area = centered_rect(50, 11, f.size()); // Adjusted height
        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1), // Weight label
                Constraint::Length(1), // Weight input
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Buttons
                Constraint::Length(1), // Spacer for error
                Constraint::Length(1), // Error message
            ])
            .split(area.inner(&ratatui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            }));

        // Label
        f.render_widget(
            Paragraph::new(format!("Target Weight ({}):", weight_unit)),
            chunks[0],
        );

        // Input Field
        let weight_style = if *focused_field == SetTargetWeightField::Weight {
            Style::default().reversed()
        } else {
            Style::default()
        };
        f.render_widget(
            Paragraph::new(weight_input.as_str()).style(weight_style),
            chunks[1],
        );

        // Buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]);

        let set_button = Paragraph::new(" Set ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Set {
                Style::default().reversed()
            } else {
                Style::default()
            });
        f.render_widget(set_button, button_layout[0]);

        let clear_button = Paragraph::new(" Clear Target ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Clear {
                Style::default().reversed()
            } else {
                Style::default()
            });
        f.render_widget(clear_button, button_layout[1]);

        let cancel_button = Paragraph::new(" Cancel ")
            .alignment(ratatui::layout::Alignment::Center)
            .style(if *focused_field == SetTargetWeightField::Cancel {
                Style::default().reversed()
            } else {
                Style::default()
            });
        f.render_widget(cancel_button, button_layout[2]);

        // Error Message
        if let Some(err) = error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
                chunks[5],
            );
        }

        // Set cursor position
        match focused_field {
            SetTargetWeightField::Weight => f.set_cursor(
                chunks[1].x + weight_input.chars().count() as u16,
                chunks[1].y,
            ),
            _ => {}
        }
    }
}

/// Helper function to create a centered rectangle for modals
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Ensure percentages are within bounds
    let percent_x = percent_x.min(100);
    let percent_y = percent_y.min(100);

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
