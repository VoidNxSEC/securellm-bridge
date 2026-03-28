//! Tool Execution Panel - Shows real-time tool execution status

use crate::themes::catppuccin::*;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

#[derive(Debug, Clone)]
pub struct ToolExecution {
    pub name: String,
    pub status: ToolStatus,
    pub output: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Idle,
    Running,
    Success,
    Failed,
}

pub struct ToolExecutionPanel {
    executions: Vec<ToolExecution>,
}

impl ToolExecutionPanel {
    pub fn new() -> Self {
        Self {
            executions: Vec::new(),
        }
    }

    /// Add or update a tool execution
    pub fn update_tool(
        &mut self,
        name: String,
        status: ToolStatus,
        output: String,
        latency_ms: u64,
    ) {
        // Find existing or add new
        if let Some(exec) = self.executions.iter_mut().find(|e| e.name == name) {
            exec.status = status;
            exec.output = output;
            exec.latency_ms = latency_ms;
        } else {
            self.executions.push(ToolExecution {
                name,
                status,
                output,
                latency_ms,
            });
        }
    }

    /// Clear all executions
    pub fn clear(&mut self) {
        self.executions.clear();
    }

    pub fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let border = if is_focused { BORDER_FOCUSED } else { BORDER };
        let title_color = if is_focused { ACCENT } else { WARNING };

        let items: Vec<ListItem> = self
            .executions
            .iter()
            .map(|exec| {
                let (icon, style) = match exec.status {
                    ToolStatus::Idle => ("○", Style::default().fg(FG_MUTED)),
                    ToolStatus::Running => (
                        "▶",
                        Style::default().fg(SECONDARY).add_modifier(Modifier::BOLD),
                    ),
                    ToolStatus::Success => (
                        "✓",
                        Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                    ),
                    ToolStatus::Failed => {
                        ("✗", Style::default().fg(ERROR).add_modifier(Modifier::BOLD))
                    }
                };

                let mut lines = vec![Line::from(vec![
                    Span::styled(icon, style.clone()),
                    Span::raw(" "),
                    Span::styled(
                        &exec.name,
                        Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                ])];

                // Show output preview (first line only)
                if !exec.output.is_empty() {
                    let preview = exec.output.lines().next().unwrap_or("");
                    let truncated = if preview.len() > 30 {
                        format!("{}...", &preview[..27])
                    } else {
                        preview.to_string()
                    };

                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(truncated, Style::default().fg(FG_MUTED)),
                    ]));
                }

                // Show latency for completed tools
                if exec.status != ToolStatus::Idle && exec.status != ToolStatus::Running {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{}ms", exec.latency_ms),
                            Style::default().fg(FG_MUTED).add_modifier(Modifier::DIM),
                        ),
                    ]));
                }

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .title(vec![
                    Span::styled("🔧 ", Style::default().fg(title_color)),
                    Span::styled(
                        format!("Tools · {}", self.executions.len()),
                        Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default().bg(BG_CARD)),
        );

        f.render_widget(list, area);
    }
}

impl Default for ToolExecutionPanel {
    fn default() -> Self {
        Self::new()
    }
}
