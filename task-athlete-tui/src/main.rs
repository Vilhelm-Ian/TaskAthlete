use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, time::Duration};
use task_athlete_lib::AppService;

// Declare modules
mod app;
mod ui;

// Use items from modules
use crate::app::App; // Get App struct from app module

fn main() -> Result<()> {
    // Initialize the library service
    let app_service = AppService::initialize().expect("Failed to initialize AppService");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(app_service);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err); // Print errors to stderr
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Ensure data is fresh before drawing (moved inside loop)
        app.refresh_data_for_active_tab(); // Refresh data *before* drawing

        terminal.draw(|f| ui::render_ui(f, app))?;

        // Poll for events with a timeout (e.g., 250ms)
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events
                if key.kind == KeyEventKind::Press {
                    // Pass key event to the app's input handler
                    // handle_key_event is now a method on App
                    app.handle_key_event(key)?;
                }
            }
            // TODO: Handle other events like resize if needed
            // if let Event::Resize(width, height) = event::read()? {
            //     // Handle resize
            // }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
