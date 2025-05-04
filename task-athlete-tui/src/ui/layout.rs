use crate::{
    app::{ActiveTab, App}, // Use App from crate::app
    ui::{
        bodyweight_tab, graphs_tab, history_tab, log_tab, modals, status_bar, tabs,
    },
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

// Main UI rendering function moved here
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

    tabs::render(f, app, main_chunks[0]);
    render_main_content(f, app, main_chunks[1]);
    status_bar::render(f, app, main_chunks[2]);

    // Render modal last if active
    if app.active_modal != crate::app::state::ActiveModal::None {
        modals::render(f, app);
    }
}

// Render the content area based on the active tab
fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let content_block = ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::NONE);
    f.render_widget(content_block, area);
    let content_area = area.inner(&ratatui::layout::Margin {
        vertical: 0,
        horizontal: 0,
    });

    match app.active_tab {
        ActiveTab::Log => log_tab::render(f, app, content_area),
        ActiveTab::History => history_tab::render(f, app, content_area),
        ActiveTab::Graphs => graphs_tab::render(f, app, content_area),
        ActiveTab::Bodyweight => bodyweight_tab::render(f, app, content_area),
    }
}

/// Helper function to create a centered rectangle with fixed dimensions.
/// Ensures the dimensions do not exceed the available screen size `r`.
pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    // Clamp dimensions to the screen size
    let clamped_width = width.min(r.width);
    let clamped_height = height.min(r.height);

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Calculate margins to center the fixed height
            Constraint::Length(r.height.saturating_sub(clamped_height) / 2),
            Constraint::Length(clamped_height), // Use the clamped fixed height
            Constraint::Length(r.height.saturating_sub(clamped_height) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            // Calculate margins to center the fixed width
            Constraint::Length(r.width.saturating_sub(clamped_width) / 2),
            Constraint::Length(clamped_width), // Use the clamped fixed width
            Constraint::Length(r.width.saturating_sub(clamped_width) / 2),
        ])
        .split(popup_layout[1])[1] // Take the middle chunk
}

// Helper function to create a centered rectangle for modals
// pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
//     let percent_x = percent_x.min(100);
//     let percent_y = percent_y.min(100);
//     let popup_layout = Layout::default()
//         .direction(Direction::Vertical)
//         .constraints([
//             Constraint::Percentage((100 - percent_y) / 2),
//             Constraint::Percentage(percent_y),
//             Constraint::Percentage((100 - percent_y) / 2),
//         ])
//         .split(r);

//     Layout::default()
//         .direction(Direction::Horizontal)
//         .constraints([
//             Constraint::Percentage((100 - percent_x) / 2),
//             Constraint::Percentage(percent_x),
//             Constraint::Percentage((100 - percent_x) / 2),
//         ])
//         .split(popup_layout[1])[1]
// }
