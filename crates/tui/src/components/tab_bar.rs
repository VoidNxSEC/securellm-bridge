//! Tab bar component for multiplex UI

use crate::themes::catppuccin::*;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Tabs},
    Frame,
};

pub struct TabBarWidget;

impl TabBarWidget {
    /// Render tab bar at the top of the screen
    pub fn render(f: &mut Frame, area: Rect, tabs: &[String], active_index: usize) {
        let titles: Vec<Span> = tabs
            .iter()
            .enumerate()
            .map(|(i, title)| {
                if i == active_index {
                    // Active tab - highlighted
                    Span::styled(
                        format!(" {} ", title),
                        Style::default()
                            .fg(BG_BASE)
                            .bg(ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    // Inactive tab - muted
                    Span::styled(
                        format!(" {} ", title),
                        Style::default().fg(FG_MUTED).bg(BG_SURFACE),
                    )
                }
            })
            .collect();

        let tabs = Tabs::new(titles)
            .select(active_index)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER))
                    .style(Style::default().bg(BG_BASE)),
            )
            .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

        f.render_widget(tabs, area);
    }
}
