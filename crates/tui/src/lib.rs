//! TUI Application for SecureLLM Bridge
//!
//! Zellij-style multiplexed interface with tabs, splits, and modern aesthetics

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::io;

mod app;
mod components;
mod input_mode;
mod multiplex;
mod themes;

pub use app::TuiApp;
pub use input_mode::InputMode;
pub use multiplex::{Pane, TabBar};
pub use themes::catppuccin::*;

use components::TabBarWidget;

/// Run the TUI application
pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = TuiApp::new()?;

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> Result<()> {
    loop {
        terminal.draw(|f| render_ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handle_input(app, key).await? {
                    break;
                }
            }
        }

        // Update app state
        app.update().await?;
    }

    Ok(())
}

fn render_ui(f: &mut Frame, app: &TuiApp) {
    let size = f.area();

    // Main layout: [Tab Bar] [Main Area] [Status Bar]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(size);

    // Render tab bar
    let tab_names: Vec<String> = app.tab_bar.tabs.iter().map(|t| t.name.clone()).collect();
    TabBarWidget::render(f, main_chunks[0], &tab_names, app.tab_bar.active_index);

    // Main workspace: [Left rail] [Center rail] [Right rail]
    let main_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(44),
            Constraint::Percentage(24),
            Constraint::Percentage(32),
        ])
        .split(main_chunks[1]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(74), Constraint::Percentage(26)])
        .split(main_columns[0]);

    let center_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(main_columns[1]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(54), Constraint::Percentage(46)])
        .split(main_columns[2]);

    app.chat_panel.render(
        f,
        left_chunks[0],
        app.focused_panel == app::FocusedPanel::Chat,
    );
    app.context_panel.render(
        f,
        left_chunks[1],
        app.focused_panel == app::FocusedPanel::Context,
    );

    if app.agent_mode {
        app.tool_panel.render(
            f,
            center_chunks[0],
            app.focused_panel == app::FocusedPanel::Tools,
        );
        app.overview_panel.render(f, center_chunks[1], app, false);
    } else {
        app.overview_panel.render(
            f,
            main_columns[1],
            app,
            app.focused_panel == app::FocusedPanel::Tools,
        );
    }

    app.task_panel.render(
        f,
        right_chunks[0],
        app.focused_panel == app::FocusedPanel::Tasks,
    );
    app.logs_panel.render(
        f,
        right_chunks[1],
        app.focused_panel == app::FocusedPanel::Logs,
    );
    app.status_bar.render(f, main_chunks[2], app);
}

async fn handle_input(app: &mut TuiApp, key: event::KeyEvent) -> Result<bool> {
    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => return Ok(true), // Quit
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('t') => {
                // Cycle tabs
                app.tab_bar.next_tab();
                return Ok(false);
            }
            _ => {}
        }
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key).await?,
        InputMode::Insert => handle_insert_mode(app, key).await?,
        InputMode::Command => handle_command_mode(app, key).await?,
        InputMode::Voice => handle_voice_mode(app, key).await?,
    }

    Ok(false)
}

async fn handle_normal_mode(app: &mut TuiApp, key: event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => {
            // Quit confirmation could be added here
        }
        KeyCode::Char('i') => {
            app.input_mode = InputMode::Insert;
        }
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
        }
        KeyCode::Char('a') => {
            app.toggle_agent_mode();
        }
        KeyCode::Char('v') => {
            app.toggle_voice().await?;
        }
        KeyCode::Tab => {
            app.cycle_focus();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_insert_mode(app: &mut TuiApp, key: event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.send_message().await?;
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_command_mode(_app: &mut TuiApp, key: event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            // Exit command mode
        }
        _ => {}
    }
    Ok(())
}

async fn handle_voice_mode(app: &mut TuiApp, key: event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('v') | KeyCode::Esc => {
            app.toggle_voice().await?;
        }
        _ => {}
    }
    Ok(())
}
