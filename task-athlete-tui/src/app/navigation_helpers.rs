// src/app/navigation_helpers.rs
use ratatui::widgets::ListState;

pub fn list_next(state: &mut ListState, list_len: usize) {
    if list_len == 0 { return; }
    let i = match state.selected() {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    state.select(Some(i));
}

pub fn list_previous(state: &mut ListState, list_len: usize) {
    if list_len == 0 { return; }
    let i = match state.selected() {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    state.select(Some(i));
}
