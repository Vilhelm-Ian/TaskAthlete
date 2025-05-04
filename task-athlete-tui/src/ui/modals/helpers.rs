use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

// --- Rendering Helpers ---

/// Renders a labeled input field and returns the area used by the input paragraph itself.
pub(super) fn render_input_field(
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

    // Render label only if it's not empty
    if !label.is_empty() {
        f.render_widget(Paragraph::new(label), chunks[0]);
    }

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
    // Use the second chunk for the input, or the first if the label is empty
    let input_chunk = if label.is_empty() {
        chunks[0]
    } else {
        chunks[1]
    };
    let text_area = input_chunk.inner(&input_margin);
    f.render_widget(Paragraph::new(value).style(input_style), text_area);

    // Return the area where the text *value* is drawn (useful for cursor positioning)
    text_area
}

/// Renders a standard horizontal pair of buttons (e.g., OK/Cancel).
pub(super) fn render_button_pair(
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
        Paragraph::new(format!(" {label1} "))
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
        Paragraph::new(format!(" {label2} "))
            .alignment(ratatui::layout::Alignment::Center)
            .style(style2),
        chunks[1],
    );
}

/// Renders an optional error message line.
pub(super) fn render_error_message(f: &mut Frame, area: Rect, error_message: Option<&String>) {
    if let Some(err) = error_message {
        f.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            area,
        );
    }
}

// Helper function to render a pair of input fields horizontally
pub(super) fn render_horizontal_input_pair(
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

/// Renders the exercise suggestions popup list below a given input area.
pub(super) fn render_exercise_suggestions_popup(
    f: &mut Frame,
    suggestions: &[String],
    list_state: &ListState,
    input_area: Rect, // The area of the exercise input field
) {
    if suggestions.is_empty() {
        return;
    }

    let suggestions_height = suggestions.len().min(5) as u16 + 2; // Limit height + borders
    let suggestions_width = input_area.width; // Match input width
    let suggestions_x = input_area.x;
    let suggestions_y = input_area.y + 1; // Position below exercise input

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

    f.render_widget(Clear, popup_area); // Clear the area behind the popup
    let mut state = list_state.clone(); // Clone for rendering
    f.render_stateful_widget(suggestions_list, popup_area, &mut state);
}

/// Renders a single centered button.
pub(super) fn render_button(f: &mut Frame, area: Rect, label: &str, is_focused: bool) {
    let base_style = Style::default().fg(Color::White);
    let style = if is_focused {
        base_style.reversed()
    } else {
        base_style
    };
    f.render_widget(
        Paragraph::new(format!(" {label} "))
            .alignment(ratatui::layout::Alignment::Center)
            .style(style),
        area,
    );
}
