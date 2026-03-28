//! Status bar component

use crate::app::TuiApp;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, app: &TuiApp) {
        use crate::themes::catppuccin::*;

        let mode_color = match app.input_mode {
            crate::InputMode::Normal => SECONDARY,    // Blue
            crate::InputMode::Insert => SUCCESS,      // Green
            crate::InputMode::Command => WARNING,     // Orange
            crate::InputMode::Voice => GRADIENT_PINK, // Pink
        };
        let provider = if app.provider_available() {
            ("online", SUCCESS)
        } else {
            ("offline", ERROR)
        };
        let voice = if app.voice_available() {
            ("voice ready", SUCCESS)
        } else {
            ("voice off", FG_MUTED)
        };
        let agent = if app.agent_mode {
            ("agent on", SUCCESS)
        } else {
            ("agent off", FG_MUTED)
        };

        let content = vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", app.input_mode.as_str()),
                    Style::default()
                        .fg(BG_BASE)
                        .bg(mode_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("◆ ", Style::default().fg(PRIMARY)),
                Span::styled("Provider ", Style::default().fg(FG_MUTED)),
                Span::styled(
                    provider.0,
                    Style::default()
                        .fg(provider.1)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("│", Style::default().fg(BORDER)),
                Span::raw("  "),
                Span::styled("◆ ", Style::default().fg(SECONDARY)),
                Span::styled("Focus ", Style::default().fg(FG_MUTED)),
                Span::styled(
                    app.focused_panel_label(),
                    Style::default()
                        .fg(GRADIENT_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("│", Style::default().fg(BORDER)),
                Span::raw("  "),
                Span::styled("◆ ", Style::default().fg(SUCCESS)),
                Span::styled("Session ", Style::default().fg(FG_MUTED)),
                Span::styled(
                    format!("{} • {}", agent.0, voice.0),
                    Style::default()
                        .fg(GRADIENT_EMERALD)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw(" "),
                Span::styled("▸ ", Style::default().fg(PRIMARY)),
                if !app.input_buffer.is_empty() {
                    Span::styled(&app.input_buffer, Style::default().fg(FG_PRIMARY))
                } else {
                    Span::styled(
                        "Tab muda foco • Ctrl+T troca aba • i escreve • a agent • v voz • Ctrl+C sai",
                        Style::default().fg(FG_MUTED).add_modifier(Modifier::ITALIC),
                    )
                },
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_CARD));

        let paragraph = Paragraph::new(content).block(block);

        f.render_widget(paragraph, area);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
