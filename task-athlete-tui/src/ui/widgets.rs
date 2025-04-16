use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Renders a paragraph within a block, applying focus style if needed.
pub fn render_paragraph<'a>(
    f: &mut Frame,
    area: Rect,
    title: &'a str,
    text: Line<'a>,
    is_focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(get_focus_style(is_focused));
    f.render_widget(Paragraph::new(text).block(block), area);
}

/// Renders an input field with optional focus style and returns the inner area for cursor positioning.
pub fn render_input_field<'a>(
    f: &mut Frame,
    area: Rect, // The entire area allocated for the input (usually one line)
    text: &'a str,
    is_focused: bool,
    show_border: bool, // Option to show a simple border around the text area
) -> Rect {
    let base_style = Style::default().fg(Color::White);
    let text_style = if is_focused {
        base_style.reversed()
    } else {
        base_style
    };
    let input_margin = Margin {
        vertical: 0,
        horizontal: 1,
    };
    let text_area = area.inner(&input_margin);

    let paragraph = Paragraph::new(text).style(text_style);

    if show_border {
        // Optionally draw a border around the text area itself
        let border_block = Block::default().borders(Borders::LEFT); // Simple left border
        f.render_widget(paragraph.block(border_block), text_area);
    } else {
        f.render_widget(paragraph, text_area);
    }

    text_area // Return the area where the text is actually drawn for cursor calculation
}

/// Renders a simple button-like paragraph with focus style.
pub fn render_button<'a>(f: &mut Frame, area: Rect, text: &'a str, is_focused: bool) {
    let base_style = Style::default().fg(Color::White);
    let style = if is_focused {
        base_style.reversed()
    } else {
        base_style
    };
    f.render_widget(
        Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(style),
        area,
    );
}

/// Renders an error message line.
pub fn render_error_message(f: &mut Frame, area: Rect, error_msg: Option<&String>) {
    if let Some(err) = error_msg {
        f.render_widget(
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red)),
            area,
        );
    }
}

/// Renders multi-line text input (like Notes)
pub fn render_textarea<'a>(
    f: &mut Frame,
    area: Rect,
    text: &'a str,
    is_focused: bool,
) -> Rect {
    let base_style = Style::default().fg(Color::White);
    let text_style = if is_focused {
        base_style.reversed()
    } else {
        base_style
    };
    let text_margin = Margin {
        vertical: 0,
        horizontal: 1,
    };
    let text_area = area.inner(&text_margin); // Area inside border

    f.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .style(text_style)
            .block(Block::default().borders(Borders::LEFT)), // Use block for border
        text_area,
    );
    text_area // Return inner area for cursor
}

/// Helper to get border style based on focus.
pub fn get_focus_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Helper for modal title border
pub fn get_modal_border_style() -> Style {
    Style::new().yellow()
}
