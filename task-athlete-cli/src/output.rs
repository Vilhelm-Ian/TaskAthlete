use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, Utc};
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use std::{collections::HashMap, io}; // Added HashMap import
use task_athlete_lib::{
    ExerciseDefinition, ExerciseStats, PbMetricInfo, Units, Workout, KM_TO_MILE,
}; // Import KM_TO_MILE from lib

// --- Helper for Table Printing ---

const EMPTY_PLACEHOLDER: &str = "-";

/// Checks if a string represents an empty cell value.
fn is_cell_empty(value: &str) -> bool {
    value.is_empty() || value == EMPTY_PLACEHOLDER
}

// --- Modified Table Printing Functions ---

/// Prints logged bodyweights in a table, hiding empty columns.
pub fn print_bodyweight_table(
    entries: &[(i64, DateTime<Utc>, f64)],
    units: Units,
    header_color: Color,
) {
    if entries.is_empty() {
        println!("No bodyweight entries found.");
        return;
    }

    let weight_unit_str = units.weight_abbr();
    let headers_str = vec![
        "Id".to_string(),
        "Timestamp (Local)".to_string(),
        format!("Weight ({weight_unit_str})"),
    ];

    let data_rows_str: Vec<Vec<String>> = entries
        .iter()
        .map(|(id, timestamp, weight)| {
            vec![
                id.to_string(),
                timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                format!("{:.2}", weight),
            ]
        })
        .collect();

    render_dynamic_table(headers_str, data_rows_str, header_color);
}

/// Prints workout entries in a formatted table, hiding empty columns.
pub fn print_workout_table(workouts: Vec<Workout>, header_color: Color, units: Units) {
    if workouts.is_empty() {
        println!("No workouts found matching the criteria.");
        return;
    }

    let weight_unit_str = units.weight_abbr();
    let distance_unit_str = units.distance_abbr();

    let headers_str = vec![
        "ID".to_string(),
        "Timestamp (Local)".to_string(),
        "Exercise".to_string(),
        "Type".to_string(),
        "Sets".to_string(),
        "Reps".to_string(),
        format!("Weight ({})", weight_unit_str),
        "Duration (min)".to_string(),
        format!("Distance ({})", distance_unit_str),
        "Notes".to_string(),
    ];

    let data_rows_str: Vec<Vec<String>> = workouts
        .into_iter()
        .map(|workout| {
            let display_distance = workout.distance.map(|km| match units {
                Units::Metric => km,
                Units::Imperial => km * KM_TO_MILE,
            });
            let weight = workout.calculate_effective_weight();

            vec![
                workout.id.to_string(),
                workout
                    .timestamp
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                workout.exercise_name,
                workout
                    .exercise_type
                    .map_or(EMPTY_PLACEHOLDER.to_string(), |t| t.to_string()),
                workout
                    .sets
                    .map_or(EMPTY_PLACEHOLDER.to_string(), |v| v.to_string()),
                workout
                    .reps
                    .map_or(EMPTY_PLACEHOLDER.to_string(), |v| v.to_string()),
                weight.map_or(EMPTY_PLACEHOLDER.to_string(), |v| format!("{v:.2}")),
                workout
                    .duration_minutes
                    .map_or(EMPTY_PLACEHOLDER.to_string(), |v| v.to_string()),
                display_distance.map_or(EMPTY_PLACEHOLDER.to_string(), |v| format!("{v:.2}")),
                workout
                    .notes
                    .as_deref()
                    .unwrap_or(EMPTY_PLACEHOLDER)
                    .to_string(), // Use placeholder
            ]
        })
        .collect();

    render_dynamic_table(headers_str, data_rows_str, header_color);
}

/// Prints exercise definitions in a formatted table, hiding empty columns.
pub fn print_exercise_definition_table(exercises: Vec<ExerciseDefinition>, header_color: Color) {
    if exercises.is_empty() {
        println!("No exercise definitions found.");
        return;
    }

    let headers_str = vec![
        "ID".to_string(),
        "Name".to_string(),
        "Type".to_string(),
        "Muscles".to_string(),
    ];

    let data_rows_str: Vec<Vec<String>> = exercises
        .into_iter()
        .map(|exercise| {
            vec![
                exercise.id.to_string(),
                exercise.name,
                exercise.type_.to_string(),
                exercise
                    .muscles
                    .as_deref()
                    .unwrap_or(EMPTY_PLACEHOLDER)
                    .to_string(), // Use placeholder
            ]
        })
        .collect();

    render_dynamic_table(headers_str, data_rows_str, header_color);
}

/// Prints aliases in a formatted table, hiding empty columns (less likely but consistent).
pub fn print_alias_table(aliases: HashMap<String, String>, header_color: Color) {
    if aliases.is_empty() {
        println!("No aliases defined.");
        return;
    }

    let headers_str = vec!["Alias".to_string(), "Canonical Exercise Name".to_string()];

    let mut sorted_aliases: Vec<_> = aliases.into_iter().collect();
    sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

    let data_rows_str: Vec<Vec<String>> = sorted_aliases
        .into_iter()
        .map(|(alias, canonical_name)| vec![alias, canonical_name])
        .collect();

    render_dynamic_table(headers_str, data_rows_str, header_color);
}

/// Prints workout volume in a table, hiding empty columns.
pub fn print_volume_table(
    volume_data: Vec<(NaiveDate, String, f64)>,
    units: Units,
    header_color: Color,
) {
    if volume_data.is_empty() {
        println!("No volume data found matching the criteria.");
        return;
    }

    let weight_unit_str = units.weight_abbr();
    let headers_str = vec![
        "Date".to_string(),
        "Exercise".to_string(),
        format!("Volume (Sets*Reps*Weight {})", weight_unit_str),
    ];

    let data_rows_str: Vec<Vec<String>> = volume_data
        .into_iter()
        .map(|(date, exercise_name, volume)| {
            vec![
                date.format("%Y-%m-%d").to_string(),
                exercise_name,
                format!("{:.2}", volume),
            ]
        })
        .collect();

    render_dynamic_table(headers_str, data_rows_str, header_color);
}

/// Generic function to render a table with dynamic column hiding.
fn render_dynamic_table(
    headers_str: Vec<String>,
    data_rows_str: Vec<Vec<String>>,
    header_color: Color,
) {
    if data_rows_str.is_empty() {
        // If there's no data, just print the headers (or a message)
        // Decide the desired behavior: print headers anyway or print a message.
        // Let's print headers for consistency with potential filtering later.
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(
                headers_str
                    .into_iter()
                    .map(|h| Cell::new(h).fg(header_color)),
            );
        println!("{table}");
        // Or: println!("No data available for this table.");
        return;
    }

    let num_cols = headers_str.len();
    let mut keep_column = vec![false; num_cols]; // Assume columns are empty until proven otherwise

    // Analyze columns: Check if any data cell in a column is non-empty
    for row in &data_rows_str {
        for (col_idx, cell_value) in row.iter().enumerate() {
            if col_idx < num_cols && !is_cell_empty(cell_value) {
                keep_column[col_idx] = true;
            }
        }
    }

    // Check if all columns were determined to be empty (unlikely if data_rows_str wasn't empty, but a safe check)
    if !keep_column.iter().any(|&keep| keep) && !data_rows_str.is_empty() {
        println!(
            "Data found, but all columns appear empty based on the placeholder '{}'.",
            EMPTY_PLACEHOLDER
        );
        // Optionally print the full table anyway, or just headers as above.
        // Let's print headers in this edge case too.
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(
                headers_str
                    .into_iter()
                    .map(|h| Cell::new(h).fg(header_color)),
            );
        println!("{table}");
        return;
    }

    // Filter headers
    let final_headers: Vec<Cell> = headers_str
        .into_iter()
        .enumerate()
        .filter_map(|(col_idx, header)| {
            if keep_column[col_idx] {
                Some(Cell::new(header).fg(header_color))
            } else {
                None
            }
        })
        .collect();

    // Filter data rows
    let final_rows: Vec<Vec<Cell>> = data_rows_str
        .into_iter()
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter_map(|(col_idx, cell_value)| {
                    if col_idx < num_cols && keep_column[col_idx] {
                        Some(Cell::new(cell_value))
                    } else {
                        None
                    }
                })
                .collect::<Vec<Cell>>()
        })
        .collect();

    // Build and print the final table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(final_headers); // Set the filtered headers

    // Add the filtered rows
    for row in final_rows {
        // Check row isn't unexpectedly empty after filtering
        if !row.is_empty() {
            table.add_row(row);
        }
    }

    println!("{table}");
}

// --- Unchanged Functions (print_exercise_stats, print_pb_message_details, CSV functions) ---
// Note: print_exercise_stats uses a key-value format for the main stats,
// so column hiding doesn't apply directly there. The PB table part already
// checks `if has_pbs`, effectively hiding the whole section if no PBs exist.
// CSV functions should generally output all columns for data integrity.

/// Prints exercise statistics.
pub fn print_exercise_stats(stats: &ExerciseStats, units: Units) {
    println!("\n--- Statistics for '{}' ---", stats.canonical_name);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    // Main stats are key-value, column hiding doesn't make sense here
    table.add_row(vec![
        Cell::new("Total Workouts").add_attribute(Attribute::Bold),
        Cell::new(stats.total_workouts.to_string()), // Use to_string for consistency
    ]);
    table.add_row(vec![
        Cell::new("First Workout").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .first_workout_date
                .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Last Workout").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .last_workout_date
                .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Avg Workouts / Week").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .avg_workouts_per_week
                .map_or("N/A".to_string(), |avg| format!("{:.2}", avg)),
        ),
    ]);
    table.add_row(vec![
        Cell::new("Longest Gap").add_attribute(Attribute::Bold),
        Cell::new(
            stats
                .longest_gap_days
                .map_or("N/A".to_string(), |gap| format!("{} days", gap)),
        ),
    ]);

    let streak_interval_str = match stats.streak_interval_days {
        1 => "(Daily)".to_string(),
        n => format!("({}-day Interval)", n),
    };
    table.add_row(vec![
        Cell::new(format!("Current Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(stats.current_streak.to_string()), // Use to_string for consistency
    ]);
    table.add_row(vec![
        Cell::new(format!("Longest Streak {}", streak_interval_str)).add_attribute(Attribute::Bold),
        Cell::new(stats.longest_streak.to_string()), // Use to_string for consistency
    ]);

    println!("{}", table);

    // Personal Bests Section - This section is already conditional
    println!("\n--- Personal Bests ---");
    let mut pb_table = Table::new();
    pb_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    let weight_unit_str = units.weight_abbr();
    let distance_unit_str = units.distance_abbr();

    let mut has_pbs = false; // This flag already handles hiding the PB table if empty
    if let Some(pb_weight) = stats.personal_bests.max_weight {
        pb_table.add_row(vec![
            Cell::new("Max Weight").add_attribute(Attribute::Bold),
            Cell::new(format!("{:.2} {}", pb_weight, weight_unit_str)),
        ]);
        has_pbs = true;
    }
    if let Some(pb_reps) = stats.personal_bests.max_reps {
        pb_table.add_row(vec![
            Cell::new("Max Reps").add_attribute(Attribute::Bold),
            Cell::new(pb_reps.to_string()), // Use to_string
        ]);
        has_pbs = true;
    }
    if let Some(pb_duration) = stats.personal_bests.max_duration_minutes {
        pb_table.add_row(vec![
            Cell::new("Max Duration").add_attribute(Attribute::Bold),
            Cell::new(format!("{} min", pb_duration)),
        ]);
        has_pbs = true;
    }
    if let Some(pb_distance_km) = stats.personal_bests.max_distance_km {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_distance_km, distance_unit_str), // clone needed
            Units::Imperial => (pb_distance_km * KM_TO_MILE, distance_unit_str), // clone needed
        };
        pb_table.add_row(vec![
            Cell::new("Max Distance").add_attribute(Attribute::Bold),
            Cell::new(format!("{dist_val:.2} {dist_unit}")),
        ]);
        has_pbs = true;
    }

    if has_pbs {
        println!("{}", pb_table);
    } else {
        println!("No personal bests recorded for this exercise yet.");
    }
    println!(); // Add a blank line at the end
}

/// Prints the formatted PB message based on achieved PBs and config settings.
/// Moved here as it's purely an output concern.
pub fn print_pb_message_details(
    pb_info: &task_athlete_lib::PBInfo,
    units: Units,
    config: &task_athlete_lib::Config,
) {
    let mut messages = Vec::new();

    // Helper to check if a PB was achieved and should be notified
    // Ensure T has Default, Copy, PartialEq traits

    if let Some((new, _old)) =
        should_display_pb(&pb_info.weight, config.pb_notifications.notify_weight)
    {
        let old_str = pb_info
            .weight
            .previous_value
            .map_or("N/A".to_string(), |v| format!("{v:.2}"));
        messages.push(format!(
            "New Max Weight: {:.2} {} (Previous: {})",
            new,
            units.weight_abbr(),
            old_str
        ));
    }

    if let Some((new, _old)) = should_display_pb(&pb_info.reps, config.pb_notifications.notify_reps)
    {
        let old_str = pb_info
            .reps
            .previous_value
            .map_or("N/A".to_string(), |v| v.to_string());
        messages.push(format!("New Max Reps: {} (Previous: {})", new, old_str));
    }

    if let Some((new, _old)) =
        should_display_pb(&pb_info.duration, config.pb_notifications.notify_duration)
    {
        let old_str = pb_info
            .duration
            .previous_value
            .map_or("N/A".to_string(), |v| format!("{} min", v));
        messages.push(format!(
            "New Max Duration: {} min (Previous: {})",
            new, old_str
        ));
    }

    if let Some((new_km, _old_km)) =
        should_display_pb(&pb_info.distance, config.pb_notifications.notify_distance)
    {
        let (new_val, unit_str) = match units {
            Units::Metric => (new_km, units.distance_abbr()),
            Units::Imperial => (new_km * KM_TO_MILE, units.distance_abbr()),
        };
        let old_str = pb_info.distance.previous_value.map_or_else(
            || "N/A".to_string(),
            |old_k| {
                let (old_v, old_u) = match units {
                    Units::Metric => (old_k, units.distance_abbr()),
                    Units::Imperial => (old_k * KM_TO_MILE, units.distance_abbr()),
                };
                format!("{:.2} {}", old_v, old_u)
            },
        );

        messages.push(format!(
            "New Max Distance: {new_val:.2} {unit_str} (Previous: {old_str})"
        ));
    }

    if !messages.is_empty() {
        // Use dynamic width for the box based on the longest message
        let max_len = messages.iter().map(String::len).max().unwrap_or(25); // Base width if no messages
        let box_width = std::cmp::max(25, max_len + 2); // Add padding

        let horizontal_line = "*".repeat(box_width + 2); // +2 for the side borders
        let title = "🎉 Personal Best! 🎉";
        let title_padding = (box_width - title.chars().count()) / 2; // Center the title roughly
        let title_line = format!(
            "*{}{}{}*",
            " ".repeat(title_padding),
            title,
            " ".repeat(box_width - title.chars().count() - title_padding)
        );

        println!("{horizontal_line}");
        println!("{title_line}");
        // println!("* {:<width$} *", "", width = box_width); // Optional empty line

        for msg in messages {
            println!("* {:<width$} *", msg, width = box_width);
        }
        println!("{}", horizontal_line);
    }
}

// --- CSV Printing Functions (Unchanged) ---
// CSV should generally preserve all columns regardless of content

pub fn print_bodyweight_csv(entries: Vec<(i64, DateTime<Utc>, f64)>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = units.weight_abbr();

    writer.write_record([
        "Id",
        "Timestamp_UTC", // Standardize on UTC for CSV
        &format!("Weight_{}", weight_unit_str),
    ])?;

    for (id, timestamp, weight) in entries {
        writer.write_record([
            id.to_string(),
            timestamp.to_rfc3339(), // Use ISO 8601/RFC3339
            format!("{:.2}", weight),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

pub fn print_workout_csv(workouts: Vec<Workout>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = units.weight_abbr();
    let distance_unit_str = units.distance_abbr();

    writer.write_record([
        "ID",
        "Timestamp_UTC", // Standardize on UTC for CSV
        "Exercise",
        "Type",
        "Sets",
        "Reps",
        &format!("Weight_{}", weight_unit_str),
        "Duration_min",
        &format!("Distance_{}", distance_unit_str),
        "Notes",
    ])?;

    for workout in workouts {
        let display_distance = workout.distance.map(|km| match units {
            Units::Metric => km,
            Units::Imperial => km * KM_TO_MILE,
        });

        writer.write_record([
            workout.id.to_string(),
            workout.timestamp.to_rfc3339(), // Use ISO 8601/RFC3339
            workout.exercise_name,
            workout
                .exercise_type
                .map_or(String::new(), |t| t.to_string()), // Use empty string for CSV nulls
            workout.sets.map_or(String::new(), |v| v.to_string()),
            workout.reps.map_or(String::new(), |v| v.to_string()),
            workout.weight.map_or(String::new(), |v| format!("{v:.2}")),
            workout
                .duration_minutes
                .map_or(String::new(), |v| v.to_string()),
            display_distance.map_or(String::new(), |v| format!("{v:.2}")),
            workout.notes.as_deref().unwrap_or("").to_string(), // Use empty string
        ])?;
    }

    writer.flush()?;
    Ok(())
}

pub fn print_alias_csv(aliases: HashMap<String, String>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(["Alias", "Canonical_Exercise_Name"])?;

    let mut sorted_aliases: Vec<_> = aliases.into_iter().collect();
    sorted_aliases.sort_by(|a, b| a.0.cmp(&b.0));

    for (alias, canonical_name) in sorted_aliases {
        writer.write_record([alias, canonical_name])?;
    }
    writer.flush()?;
    Ok(())
}

pub fn print_volume_csv(volume_data: Vec<(NaiveDate, String, f64)>, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    let weight_unit_str = units.weight_abbr();

    writer.write_record([
        "Date",
        "Exercise",
        &format!("Volume_Sets*Reps*Weight_{weight_unit_str}"),
    ])?;

    for (date, exercise_name, volume) in volume_data {
        writer.write_record([
            date.format("%Y-%m-%d").to_string(),
            exercise_name,
            format!("{volume:.2}"),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

pub fn print_stats_csv(stats: &ExerciseStats, units: Units) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(["Statistic", "Value"])?;

    writer.write_record(["Exercise_Name", &stats.canonical_name])?;
    writer.write_record(["Total_Workouts", &stats.total_workouts.to_string()])?;
    writer.write_record([
        "First_Workout",
        &stats
            .first_workout_date
            .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
    ])?;
    writer.write_record([
        "Last_Workout",
        &stats
            .last_workout_date
            .map_or("N/A".to_string(), |d| d.format("%Y-%m-%d").to_string()),
    ])?;
    writer.write_record([
        "Avg_Workouts_Per_Week",
        &stats
            .avg_workouts_per_week
            .map_or("N/A".to_string(), |avg| format!("{avg:.2}")),
    ])?;
    writer.write_record([
        "Longest_Gap_Days",
        &stats
            .longest_gap_days
            .map_or("N/A".to_string(), |gap| gap.to_string()),
    ])?;
    writer.write_record([
        "Streak_Interval_Days",
        &stats.streak_interval_days.to_string(),
    ])?;
    writer.write_record(["Current_Streak", &stats.current_streak.to_string()])?;
    writer.write_record(["Longest_Streak", &stats.longest_streak.to_string()])?;

    let weight_unit_str = units.weight_abbr();
    let distance_unit_str = units.distance_abbr();

    if let Some(pb_weight) = stats.personal_bests.max_weight {
        writer.write_record([
            &format!("PB_Max_Weight_{weight_unit_str}"),
            &format!("{pb_weight:.2}"),
        ])?;
    } else {
        writer.write_record([&format!("PB_Max_Weight_{weight_unit_str}"), ""])?;
        // Empty if no PB
    }

    if let Some(pb_reps) = stats.personal_bests.max_reps {
        writer.write_record(["PB_Max_Reps", &pb_reps.to_string()])?;
    } else {
        writer.write_record(["PB_Max_Reps", ""])?;
    }

    if let Some(pb_duration) = stats.personal_bests.max_duration_minutes {
        writer.write_record(["PB_Max_Duration_min", &pb_duration.to_string()])?;
    } else {
        writer.write_record(["PB_Max_Duration_min", ""])?;
    }

    if let Some(pb_distance_km) = stats.personal_bests.max_distance_km {
        let (dist_val, dist_unit) = match units {
            Units::Metric => (pb_distance_km, distance_unit_str),
            Units::Imperial => (pb_distance_km * KM_TO_MILE, distance_unit_str),
        };
        writer.write_record([
            &format!("PB_Max_Distance_{}", dist_unit),
            &format!("{:.2}", dist_val),
        ])?;
    } else {
        writer.write_record([&format!("PB_Max_Distance_{distance_unit_str}"), ""])?;
    }

    writer.flush()?;
    Ok(())
}

pub fn print_exercise_definition_csv(exercises: Vec<ExerciseDefinition>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    writer.write_record(["ID", "Name", "Type", "Muscles"])?;

    for exercise in exercises {
        writer.write_record([
            exercise.id.to_string(),
            exercise.name,
            exercise.type_.to_string(),
            exercise.muscles.as_deref().unwrap_or("").to_string(), // Use empty string
        ])?;
    }
    writer.flush()?;
    Ok(())
}

fn should_display_pb<T>(info: &PbMetricInfo<T>, notify_enabled: bool) -> Option<(T, T)>
where
    T: Default + Copy + PartialEq,
{
    if info.achieved && notify_enabled {
        // Check if new value exists, provide default otherwise
        let new_val = info.new_value.unwrap_or_default();
        // Check if previous value exists, provide default otherwise
        let prev_val = info.previous_value.unwrap_or_default();
        // Only show if new is different from old (handles initial PB case where old might be default 0)
        if new_val != prev_val || info.previous_value.is_none() {
            // Also show if it's the very first PB
            Some((new_val, prev_val))
        } else {
            None
        }
    } else {
        None
    }
}
