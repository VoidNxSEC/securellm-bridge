//! Session overview / command center panel

use crate::{themes::catppuccin::*, TuiApp};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct OverviewPanel;

impl OverviewPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, area: Rect, app: &TuiApp, is_focused: bool) {
        let border = if is_focused { BORDER_FOCUSED } else { BORDER };
        let title_color = if is_focused { ACCENT } else { PRIMARY };
        let voice = if app.voice_available() {
            ("online", SUCCESS)
        } else {
            ("offline", FG_MUTED)
        };
        let agent = if app.agent_mode {
            ("enabled", SUCCESS)
        } else {
            ("standby", WARNING)
        };

        let content = vec![
            Line::from(vec![
                Span::styled("Session", Style::default().fg(FG_MUTED)),
                Span::raw("  "),
                Span::styled(
                    format!("{} messages", app.message_count()),
                    Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Focus", Style::default().fg(FG_MUTED)),
                Span::raw("    "),
                Span::styled(
                    app.focused_panel_label(),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Agent", Style::default().fg(FG_MUTED)),
                Span::raw("    "),
                Span::styled(
                    agent.0,
                    Style::default().fg(agent.1).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Voice", Style::default().fg(FG_MUTED)),
                Span::raw("    "),
                Span::styled(
                    voice.0,
                    Style::default().fg(voice.1).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Flow", Style::default().fg(GRADIENT_PURPLE)),
                Span::raw("  "),
                Span::styled(
                    "Tab cycle focus  •  i compose  •  Enter send  •  a toggle agent",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled("Hints", Style::default().fg(GRADIENT_ORANGE)),
                Span::raw(" "),
                Span::styled(
                    if app.input_buffer.is_empty() {
                        "Use this column as command center while the tool panel is hidden."
                    } else {
                        "Draft is live in the status bar. Enter sends from insert mode."
                    },
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(content).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(vec![
                    Span::styled("◈ ", Style::default().fg(title_color)),
                    Span::styled(
                        "Command Center",
                        Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(BG_CARD)),
        );

        f.render_widget(paragraph, area);
    }
}

impl Default for OverviewPanel {
    fn default() -> Self {
        Self::new()
    }
}
