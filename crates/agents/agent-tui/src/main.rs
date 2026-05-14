//! Agent TUI - Floating overlay for agentic shell interaction
//!
//! Minimal, borderless interface that:
//! - Appears via Super+K (Hyprland keybind)
//! - Takes natural language input
//! - Generates shell commands via LLM
//! - Injects into Zellij session
//! - Disappears after execution

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

// Catppuccin Mocha
const BG_BASE: Color = Color::Rgb(30, 30, 46);
const BG_ELEVATED: Color = Color::Rgb(52, 55, 76);
const BG_GLASS: Color = Color::Rgb(36, 38, 52);
const ACCENT: Color = Color::Rgb(137, 180, 250); // Blue
const DIM: Color = Color::Rgb(108, 112, 134); // Overlay0
const BRIGHT: Color = Color::Rgb(205, 214, 244); // Text
const MAUVE: Color = Color::Rgb(203, 166, 247);
const SUCCESS: Color = Color::Rgb(166, 227, 161); // Green
const WARNING: Color = Color::Rgb(249, 226, 175); // Yellow
const ERROR: Color = Color::Rgb(243, 139, 168); // Red

struct AliasHelper {
    trigger: &'static str,
    helper: &'static str,
    action: HelperAction,
}

enum HelperAction {
    DirectCommand(&'static str),
    QueryRecipe,
    PromptExpansion(&'static str),
}

const ALIAS_HELPERS: &[AliasHelper] = &[
    AliasHelper {
        trigger: "@health",
        helper: "just health",
        action: HelperAction::DirectCommand("just health"),
    },
    AliasHelper {
        trigger: "@info",
        helper: "just info",
        action: HelperAction::DirectCommand("just info"),
    },
    AliasHelper {
        trigger: "@test",
        helper: "just test",
        action: HelperAction::DirectCommand("just test"),
    },
    AliasHelper {
        trigger: "@lint",
        helper: "just lint",
        action: HelperAction::DirectCommand("just lint"),
    },
    AliasHelper {
        trigger: "@format",
        helper: "just format",
        action: HelperAction::DirectCommand("just format"),
    },
    AliasHelper {
        trigger: "@check",
        helper: "just quality",
        action: HelperAction::DirectCommand("just quality"),
    },
    AliasHelper {
        trigger: "@pipeline",
        helper: "just pipeline",
        action: HelperAction::DirectCommand("just pipeline"),
    },
    AliasHelper {
        trigger: "@query",
        helper: "just query \"<question>\"",
        action: HelperAction::QueryRecipe,
    },
    AliasHelper {
        trigger: "@fix",
        helper: "context-aware safe remediation",
        action: HelperAction::PromptExpansion(
            "diagnose the issue visible in the current shell context and produce a single safe command to move toward a fix",
        ),
    },
];

/// Load context directly from Zellij pane
fn load_shell_context() -> String {
    // 1. Get current pane content (dump-screen)
    let output = Command::new("zellij")
        .args(["action", "dump-screen", "--full"])
        .output();

    let screen_content = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => String::from("[No Zellij context available]"),
    };

    // Take last 20 lines to avoid too much context
    let context_lines: Vec<&str> = screen_content.lines().rev().take(20).collect();
    let recent_context: Vec<&str> = context_lines.into_iter().rev().collect();

    format!(
        "Terminal content (last 20 lines):\n---\n{}\n---\nCurrent directory: {}",
        recent_context.join("\n"),
        std::env::current_dir().unwrap_or_default().display()
    )
}

struct AgentOverlay {
    input: String,
    output: String,
    submitted_prompt: String,
    status: Status,
    should_quit: bool,
    animation_tick: usize,
    last_tick: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Idle,
    Processing,
    Ready,
    Error,
}

impl AgentOverlay {
    fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            submitted_prompt: String::new(),
            status: Status::Idle,
            should_quit: false,
            animation_tick: 0,
            last_tick: Instant::now(),
        }
    }

    async fn process_input(&mut self) -> Result<()> {
        if self.input.is_empty() {
            return Ok(());
        }

        self.submitted_prompt = self.input.clone();
        if let Some(command) = self.direct_command_for_input() {
            self.output = command;
            self.status = Status::Ready;
            return Ok(());
        }

        self.status = Status::Processing;
        self.output = "Thinking through the shell context...".to_string();
        let expanded_input = self.expanded_input();

        // Call LLM
        use securellm_core::{LLMProvider, Message, MessageContent, MessageRole, Request};
        use securellm_providers::llamacpp::LlamaCppProvider;

        match LlamaCppProvider::new(8081, "llamacppturbo") {
            Ok(provider) => {
                let context = load_shell_context();
                let system_prompt = format!(
                    "You are a shell command generator. Given a natural language request, output ONLY the shell command to execute. No explanations, no markdown, just the raw command.\n\n{}\n\nRespond with the exact command to run.",
                    context
                );

                let request = Request::new("llamacpp", "llamacppturbo")
                    .add_message(Message {
                        role: MessageRole::System,
                        content: MessageContent::Text(system_prompt),
                        name: None,
                        metadata: None,
                    })
                    .add_message(Message {
                        role: MessageRole::User,
                        content: MessageContent::Text(expanded_input),
                        name: None,
                        metadata: None,
                    });

                match provider.send_request(request).await {
                    Ok(response) => {
                        if let Ok(text) = response.text() {
                            self.output = text.trim().to_string();
                            self.status = Status::Ready;
                        }
                    }
                    Err(e) => {
                        self.output = format!("Error: {}", e);
                        self.status = Status::Error;
                    }
                }
            }
            Err(e) => {
                self.output = format!("LLM Error: {}", e);
                self.status = Status::Error;
            }
        }

        Ok(())
    }

    fn inject_to_zellij(&self) -> Result<()> {
        if self.output.is_empty() || self.status != Status::Ready {
            return Ok(());
        }

        // Check if Zellij is running
        let zellij_check = Command::new("zellij").args(["list-sessions"]).output();

        match zellij_check {
            Ok(output) if output.status.success() => {
                let sessions = String::from_utf8_lossy(&output.stdout);
                if sessions.trim().is_empty() {
                    // No Zellij session, just print command
                    eprintln!("No Zellij session. Command: {}", self.output);
                    return Ok(());
                }

                // Inject command into active Zellij pane
                let cmd = format!("{}\n", self.output);
                let result = Command::new("zellij")
                    .args(["action", "write-chars", &cmd])
                    .status();

                if let Err(e) = result {
                    eprintln!("Zellij injection failed: {}", e);
                }
            }
            _ => {
                // Zellij not available, copy to clipboard as fallback
                let _ = Command::new("wl-copy").arg(&self.output).status();
                eprintln!("Copied to clipboard: {}", self.output);
            }
        }

        Ok(())
    }

    fn status_title(&self) -> &'static str {
        match self.status {
            Status::Idle => "Ready",
            Status::Processing => "Generating",
            Status::Ready => "Command Ready",
            Status::Error => "Needs Review",
        }
    }

    fn status_hint(&self) -> &'static str {
        match self.status {
            Status::Idle => "Enter generates a command. Esc closes the overlay.",
            Status::Processing => "Generating command from current terminal context...",
            Status::Ready => "Enter injects into Zellij. Keep typing to refine.",
            Status::Error => "Edit the prompt and press Enter to retry.",
        }
    }

    fn status_color(&self) -> Color {
        match self.status {
            Status::Idle => ACCENT,
            Status::Processing => WARNING,
            Status::Ready => SUCCESS,
            Status::Error => ERROR,
        }
    }

    fn intent_text(&self) -> &str {
        if self.submitted_prompt.is_empty() {
            "Describe the operation you want to run."
        } else {
            self.submitted_prompt.as_str()
        }
    }

    fn command_text(&self) -> &str {
        if self.output.is_empty() {
            "Generated shell command will appear here."
        } else {
            self.output.as_str()
        }
    }

    fn active_alias(&self) -> Option<&'static AliasHelper> {
        let alias = self.input.split_whitespace().next()?;
        ALIAS_HELPERS.iter().find(|helper| helper.trigger == alias)
    }

    fn expanded_input(&self) -> String {
        Self::expand_alias(self.input.as_str())
    }

    fn direct_command_for_input(&self) -> Option<String> {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let first = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or("").trim();
        let alias = ALIAS_HELPERS
            .iter()
            .find(|helper| helper.trigger == first)?;

        match alias.action {
            HelperAction::DirectCommand(command) => Some(command.to_string()),
            HelperAction::QueryRecipe => {
                if rest.is_empty() {
                    Some("just query \"your question here\"".to_string())
                } else {
                    Some(format!("just query {}", shell_quote(rest)))
                }
            }
            HelperAction::PromptExpansion(_) => None,
        }
    }

    fn expand_alias(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let first = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or("").trim();

        if let Some(alias) = ALIAS_HELPERS.iter().find(|helper| helper.trigger == first) {
            match alias.action {
                HelperAction::PromptExpansion(expansion) => {
                    if rest.is_empty() {
                        expansion.to_string()
                    } else {
                        format!("{}: {}", expansion, rest)
                    }
                }
                HelperAction::DirectCommand(command) => command.to_string(),
                HelperAction::QueryRecipe => {
                    if rest.is_empty() {
                        "run the project's knowledge-base query helper from the Justfile"
                            .to_string()
                    } else {
                        format!(
                            "run the project's knowledge-base query helper from the Justfile with this question: {}",
                            rest
                        )
                    }
                }
            }
        } else {
            trimmed.to_string()
        }
    }

    fn plan_summary(&self) -> Vec<Line<'static>> {
        let phase = match self.status {
            Status::Idle => "Awaiting intent",
            Status::Processing => match self.animation_tick % 3 {
                0 => "Interpreting scope",
                1 => "Building command artifact",
                _ => "Verifying active shell context",
            },
            Status::Ready => "Command prepared for execution",
            Status::Error => "Generation failed, awaiting refinement",
        };

        let action = if self.submitted_prompt.is_empty() {
            "No command requested yet."
        } else if self.direct_command_for_input().is_some() {
            "Resolved directly to a project helper command."
        } else if self.status == Status::Ready {
            "Command is ready to inject into the active shell."
        } else if self.status == Status::Processing {
            "Resolving current shell context before emitting a single command."
        } else if self.status == Status::Error {
            "Review the generated error and clarify the request."
        } else {
            "Press Enter to translate the intent into a shell command."
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Phase", Style::default().fg(DIM)),
                Span::raw("  "),
                Span::styled(
                    phase,
                    Style::default()
                        .fg(self.status_color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Scope", Style::default().fg(DIM)),
                Span::raw("  "),
                Span::styled(
                    if is_inside_zellij() {
                        "Active Zellij session"
                    } else {
                        "Standalone terminal session"
                    },
                    Style::default().fg(BRIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Action", Style::default().fg(DIM)),
                Span::raw(" "),
                Span::styled(action, Style::default().fg(BRIGHT)),
            ]),
        ];

        if let Some(alias) = self.active_alias() {
            lines.push(Line::from(vec![
                Span::styled("Alias", Style::default().fg(DIM)),
                Span::raw("  "),
                Span::styled(
                    format!("{} → {}", alias.trigger, alias.helper),
                    Style::default().fg(ACCENT),
                ),
            ]));
        }

        lines
    }

    fn tick(&mut self) {
        if self.last_tick.elapsed() >= Duration::from_millis(160) {
            self.animation_tick = self.animation_tick.wrapping_add(1);
            self.last_tick = Instant::now();
        }
    }

    fn activity_glyph(&self) -> &'static str {
        match self.status {
            Status::Processing => match self.animation_tick % 4 {
                0 => "·",
                1 => "•",
                2 => "●",
                _ => "•",
            },
            Status::Ready => "●",
            Status::Error => "•",
            Status::Idle => "·",
        }
    }

    fn animated_command_text(&self) -> String {
        match self.status {
            Status::Processing => {
                let dots = ".".repeat((self.animation_tick % 3) + 1);
                format!("Generating shell command{}", dots)
            }
            _ => self.command_text().to_string(),
        }
    }

    fn animated_hint(&self) -> String {
        match self.status {
            Status::Processing => {
                let frames = ["reading shell", "mapping intent", "assembling command"];
                frames[self.animation_tick % frames.len()].to_string()
            }
            _ => {
                if let Some(alias) = self.active_alias() {
                    match alias.action {
                        HelperAction::DirectCommand(_) | HelperAction::QueryRecipe => {
                            format!("{} resolves to: {}", alias.trigger, alias.helper)
                        }
                        HelperAction::PromptExpansion(_) => {
                            format!("{} expands to: {}", alias.trigger, alias.helper)
                        }
                    }
                } else {
                    self.status_hint().to_string()
                }
            }
        }
    }
}

/// Check if we're running inside Zellij
fn is_inside_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

/// Get current Zellij session name
fn get_zellij_session() -> Option<String> {
    std::env::var("ZELLIJ_SESSION_NAME").ok()
}

fn shell_quote(input: &str) -> String {
    format!("\"{}\"", input.replace('\\', "\\\\").replace('"', "\\\""))
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}

fn render(f: &mut Frame, app: &AgentOverlay) {
    let area = f.area();
    let overlay = centered_rect(area, 70, 76);
    f.render_widget(Clear, overlay);

    let shell_label = if is_inside_zellij() {
        get_zellij_session().unwrap_or_else(|| "zellij".to_string())
    } else {
        "standalone".to_string()
    };
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "/".to_string());

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BG_ELEVATED))
        .style(Style::default().bg(BG_BASE))
        .title(Line::from(vec![
            Span::styled(
                " Command Center ",
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("•", Style::default().fg(DIM)),
            Span::styled(
                format!(" {} ", app.status_title()),
                Style::default()
                    .fg(app.status_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(
                "Enter",
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" generate/execute", Style::default().fg(DIM)),
            Span::styled("  •  ", Style::default().fg(DIM)),
            Span::styled(
                "Esc",
                Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" close", Style::default().fg(DIM)),
        ]));
    f.render_widget(outer, overlay);

    let inner = overlay.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(5),
        ])
        .split(inner);

    let context_strip = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            shell_label,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(DIM)),
        Span::styled(cwd, Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled("  •  ", Style::default().fg(DIM)),
        Span::styled(
            app.activity_glyph(),
            Style::default().fg(app.status_color()),
        ),
        Span::styled(" ", Style::default().fg(DIM)),
        Span::styled(app.status_title(), Style::default().fg(app.status_color())),
        Span::styled(
            "  •  low-latency command translation",
            Style::default().fg(DIM),
        ),
    ])])
    .style(Style::default().bg(BG_BASE));
    f.render_widget(context_strip, chunks[0]);

    let intent_panel = Paragraph::new(app.intent_text())
        .style(Style::default().fg(BRIGHT))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(BG_ELEVATED))
                .style(Style::default().bg(BG_BASE))
                .title(Line::from(vec![
                    Span::styled(
                        "Intent",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  user request", Style::default().fg(DIM)),
                ])),
        );
    f.render_widget(intent_panel, chunks[1]);

    let plan_panel = Paragraph::new(app.plan_summary())
        .style(Style::default().fg(DIM))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(BG_ELEVATED))
                .style(Style::default().bg(BG_BASE))
                .title(Line::from(vec![
                    Span::styled(
                        "Plan",
                        Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  execution transparency", Style::default().fg(DIM)),
                ])),
        );
    f.render_widget(plan_panel, chunks[2]);

    let command_panel = Paragraph::new(app.animated_command_text())
        .style(
            Style::default()
                .fg(match app.status {
                    Status::Ready => SUCCESS,
                    Status::Error => ERROR,
                    Status::Processing => WARNING,
                    Status::Idle => BRIGHT,
                })
                .add_modifier(Modifier::BOLD),
        )
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BG_ELEVATED))
                .style(Style::default().bg(BG_GLASS))
                .title(Line::from(vec![
                    Span::styled(
                        "Command",
                        Style::default()
                            .fg(app.status_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  exact shell artifact", Style::default().fg(DIM)),
                ])),
        );
    f.render_widget(command_panel, chunks[3]);

    let composer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "Input",
                Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  refine intent inline", Style::default().fg(DIM)),
        ]),
        Line::from(vec![
            Span::styled(
                "› ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            if app.input.is_empty() {
                Span::styled(
                    "Ask for a shell action...",
                    Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
                )
            } else {
                Span::styled(app.input.as_str(), Style::default().fg(BRIGHT))
            },
        ]),
        Line::from(vec![
            Span::styled("@health", Style::default().fg(ACCENT)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled("@info", Style::default().fg(ACCENT)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled("@test", Style::default().fg(ACCENT)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled("@check", Style::default().fg(ACCENT)),
            Span::styled("  ", Style::default().fg(DIM)),
            Span::styled("@query", Style::default().fg(ACCENT)),
        ]),
        Line::from(Span::styled(app.animated_hint(), Style::default().fg(DIM))),
    ])
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BG_ELEVATED))
            .style(Style::default().bg(BG_GLASS)),
    );
    f.render_widget(composer, chunks[4]);

    let cursor_x = chunks[4].x + 4 + app.input.len() as u16;
    let cursor_y = chunks[4].y + 2;
    f.set_cursor_position((cursor_x, cursor_y));
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AgentOverlay,
) -> Result<()> {
    loop {
        app.tick();
        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        app.should_quit = true;
                        break;
                    }
                    KeyCode::Enter => {
                        if app.status == Status::Ready {
                            // Inject and quit
                            app.inject_to_zellij()?;
                            app.should_quit = true;
                            break;
                        } else {
                            // Process input
                            app.process_input().await?;
                        }
                    }
                    KeyCode::Char(c) => {
                        if app.status != Status::Processing {
                            app.input.push(c);
                            app.status = Status::Idle;
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        app.status = Status::Idle;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let mut app = AgentOverlay::new();
    let result = run_app(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
