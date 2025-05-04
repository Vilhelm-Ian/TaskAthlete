// src/ui/modals/pb_modal.rs
use crate::{
    app::{
        state::{ActiveModal, PbModalField},
        App,
    },
    ui::layout::centered_rect,
    ui::modals::helpers, // Use helpers module
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Span, Text}, // Use Text
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use task_athlete_lib::Units; // Import Units

pub(super) fn render(f: &mut Frame, app: &App) {
    if let ActiveModal::PersonalBest {
        exercise_name,
        pb_info,
        focused_field,
    } = &app.active_modal
    {
        let block = Block::default()
            .title("🎉 Personal Best! 🎉")
            .borders(Borders::ALL)
            .border_style(Style::new().yellow().bold());

        let mut text_lines = vec![Line::from(Span::styled(
            exercise_name.as_str(),
            Style::default().bold().underlined(),
        ))];
        text_lines.push(Line::from(" ")); // Spacer

        let units = &app.service.config.units;
        let (weight_unit, dist_unit) = match units {
            Units::Metric => ("kg", "km"),
            Units::Imperial => ("lbs", "mi"),
        };

        if pb_info.weight.achieved {
            let weight_val = pb_info.weight.new_value.unwrap_or(0.0);
            let display_weight = match units {
                Units::Metric => weight_val,
                Units::Imperial => weight_val * 2.20462,
            };
            text_lines.push(Line::from(format!(
                "- New Max Weight: {:.1} {}",
                display_weight, weight_unit
            )));
        }
        if pb_info.reps.achieved {
            text_lines.push(Line::from(format!(
                "- New Max Reps: {}",
                pb_info.reps.new_value.unwrap_or(0)
            )));
        }
        if pb_info.duration.achieved {
            text_lines.push(Line::from(format!(
                "- New Max Duration: {} min",
                pb_info.duration.new_value.unwrap_or(0)
            )));
        }
        if pb_info.distance.achieved {
            let dist_val = pb_info.distance.new_value.unwrap_or(0.0);
            let display_dist = match units {
                Units::Metric => dist_val,
                Units::Imperial => dist_val * 0.621371,
            };
            text_lines.push(Line::from(format!(
                "- New Max Distance: {:.1} {}",
                display_dist, dist_unit
            )));
        }

        text_lines.push(Line::from(" ")); // Spacer

        let content_height = text_lines.len() as u16;
        let modal_height = content_height + 4; // Content + title/border + button + bottom border
        let modal_width = 50; // Fixed width

        let area = centered_rect(modal_width, modal_height, f.size());

        f.render_widget(Clear, area); // Clear background
        f.render_widget(block, area);

        let inner_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(inner_area); // Simplified layout

        f.render_widget(Paragraph::new(Text::from(text_lines)), chunks[0]);

        helpers::render_button(f, chunks[1], "OK", *focused_field == PbModalField::OkButton);
    }
}
