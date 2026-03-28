//! Chat panel component

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub struct ChatPanel {
    messages: Vec<ChatMessage>,
}

#[derive(Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "Welcome to SecureLLM Bridge TUI. Press 'i' to enter insert mode, 'v' for voice, ':' for commands.".to_string(),
                }
            ],
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        use crate::themes::catppuccin::*;

        let border = if is_focused { BORDER_FOCUSED } else { BORDER };
        let title_color = if is_focused { ACCENT } else { PRIMARY };

        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|msg| {
                let (role_icon, role_style) = match msg.role.as_str() {
                    "user" => (
                        "▶",
                        Style::default()
                            .fg(GRADIENT_BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    "assistant" => (
                        "◆",
                        Style::default()
                            .fg(GRADIENT_PURPLE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    "system" => ("●", Style::default().fg(WARNING)),
                    _ => ("○", Style::default().fg(FG_MUTED)),
                };

                let content = Line::from(vec![
                    Span::styled(role_icon, role_style.clone()),
                    Span::raw(" "),
                    Span::styled(
                        format!("{} ", msg.role),
                        role_style.add_modifier(Modifier::DIM),
                    ),
                    Span::styled(&msg.content, Style::default().fg(FG_PRIMARY)),
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(vec![
                    Span::styled("💬 ", Style::default().fg(title_color)),
                    Span::styled(
                        format!("Chat · {}", self.messages.len()),
                        Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(BG_CARD)),
        );

        f.render_widget(list, area);
    }
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self::new()
    }
}
