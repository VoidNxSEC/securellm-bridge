//! Pane abstraction for multiplex TUI

use crate::components::{ChatPanel, ToolExecutionPanel};
use ratatui::{layout::Rect, Frame};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub Uuid);

impl PaneId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Pane types for multiplexing
pub enum Pane {
    Chat(ChatPane),
    Tools(ToolPane),
}

/// Chat pane with agent support
pub struct ChatPane {
    pub id: PaneId,
    pub panel: ChatPanel,
    pub agent_enabled: bool,
}

impl ChatPane {
    pub fn new() -> Self {
        Self {
            id: PaneId::new(),
            panel: ChatPanel::new(),
            agent_enabled: false,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.panel.render(f, area, false);
    }
}

/// Tool execution pane
pub struct ToolPane {
    pub id: PaneId,
    pub panel: ToolExecutionPanel,
}

impl ToolPane {
    pub fn new() -> Self {
        Self {
            id: PaneId::new(),
            panel: ToolExecutionPanel::new(),
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.panel.render(f, area, false);
    }
}

impl Pane {
    pub fn id(&self) -> PaneId {
        match self {
            Pane::Chat(p) => p.id,
            Pane::Tools(p) => p.id,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        match self {
            Pane::Chat(p) => p.render(f, area),
            Pane::Tools(p) => p.render(f, area),
        }
    }
}
