// task-athlete-tui/src/ui/mod.rs

// Declare UI component modules
mod bodyweight_tab;
mod layout;
mod log_tab;
mod modals;
mod placeholders;
mod status_bar;
mod tabs;

// Re-export the main render function
pub use layout::render_ui; // Assuming render_ui is moved to layout.rs or stays here
