// src/app/navigation_helpers.rs
use ratatui::widgets::ListState;

pub fn list_next(state: &mut ListState, list_len: usize) {
    if list_len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i >= list_len - 1 => 0,
        Some(i) => i + 1,
        None => 0,
    };
    state.select(Some(i));
}

pub fn list_previous(state: &mut ListState, list_len: usize) {
    if list_len == 0 {
        return;
    }
    let i = match state.selected() {
        Some(i) if i == 0 => list_len - 1,
        Some(i) => i - 1,
        None => list_len.saturating_sub(1),
    };
    state.select(Some(i));
}

/// Ensures the selection index is valid for the given list length.
/// Selects the last item if the index is out of bounds, or None if the list is empty.
/// Selects the first item (0) if currently None and the list is not empty.
pub fn ensure_selection_is_valid(state: &mut ListState, list_len: usize) {
    if list_len == 0 {
        state.select(None);
    } else {
        match state.selected() {
            Some(i) if i >= list_len => state.select(Some(list_len - 1)),
            None => state.select(Some(0)), // Select first item if None
            _ => {}                        // Current selection is valid
        }
    }
}
