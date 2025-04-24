// src/app/utils.rs
use super::AppInputError;
use chrono::{Duration, NaiveDate, Utc};
use std::str::FromStr;

// --- Parsing Helpers ---

pub fn parse_optional_int<T: FromStr>(input: &str) -> Result<Option<T>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<T>()
            .map(Some)
            .map_err(|_| AppInputError::InvalidNumber(trimmed.to_string()))
        // Add validation like non-negative if needed, depends on T
    }
}

pub fn parse_optional_float(input: &str) -> Result<Option<f64>, AppInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<f64>()
            .map_err(|_| AppInputError::InvalidNumber(trimmed.to_string()))
            .and_then(|f| {
                if f >= 0.0 {
                    Ok(Some(f))
                } else {
                    Err(AppInputError::InvalidNumber(
                        "Value cannot be negative".to_string(),
                    ))
                }
            })
    }
}

pub fn parse_modal_date(date_str: &str) -> Result<NaiveDate, AppInputError> {
    let trimmed = date_str.trim().to_lowercase();
    match trimmed.as_str() {
        "today" | "" => Ok(Utc::now().date_naive()), // Default to today if empty
        "yesterday" | "y" => Ok(Utc::now().date_naive() - Duration::days(1)),
        _ => NaiveDate::parse_from_str(&trimmed, "%Y-%m-%d")
            .map_err(|_| AppInputError::InvalidDate(date_str.to_string())),
    }
}

pub fn parse_modal_weight(weight_str: &str) -> Result<f64, AppInputError> {
    let trimmed = weight_str.trim();
    if trimmed.is_empty() {
        return Err(AppInputError::InputEmpty);
    }
    trimmed
        .parse::<f64>()
        .map_err(|e| AppInputError::InvalidNumber(e.to_string()))
        .and_then(|w| {
            if w > 0.0 {
                Ok(w)
            } else {
                Err(AppInputError::InvalidNumber(
                    "Weight must be positive".to_string(),
                ))
            }
        })
}

// --- Input Modification ---

pub fn modify_numeric_input<T>(input_str: &mut String, delta: T, min_val: Option<T>, is_float: bool)
where
    T: FromStr
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy
        + std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Debug, // Ensure FromStr error is Debug
{
    let mut num_val: T = match input_str.parse::<T>() {
        Ok(v) => v,
        // If parse fails, try to start from 0 or a reasonable default? Or just return.
        // Let's start from 0 if parse fails or string is empty.
        Err(_) => match "0".parse::<T>() {
            Ok(zero) => zero,
            Err(_) => return, // Cannot even parse "0" for type T
        },
    };

    num_val = num_val + delta; // Apply delta

    // Apply minimum value constraint
    if let Some(min) = min_val {
        if num_val < min {
            num_val = min;
        }
    }

    // Update the string
    if is_float {
        // Find the number of decimal places in the delta to format nicely
        let delta_str = format!("{}", delta);
        let decimals = delta_str.split('.').nth(1).map_or(0, |s| s.len());
        *input_str = format!("{:.prec$}", num_val, prec = decimals.max(1)); // Format floats nicely
    } else {
        *input_str = num_val.to_string();
    }
}

pub fn parse_option_to_input<T>(option: Option<T>) -> String
where
    T: std::fmt::Display,
{
    if let Some(s) = option {
        format!("{}", s)
    } else {
        String::new()
    }
}
