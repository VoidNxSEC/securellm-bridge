//! Layout engine for multiplex splits

use super::pane::PaneId;
use ratatui::layout::{Constraint, Direction, Layout as RatatuiLayout, Rect};

/// Layout tree for pane organization
#[derive(Clone)]
pub enum Layout {
    /// Single pane
    Single(PaneId),

    /// Horizontal split (left | right)
    HSplit {
        left: Box<Layout>,
        right: Box<Layout>,
        ratio: u16, // Percentage for left (0-100)
    },

    /// Vertical split (top / bottom)
    VSplit {
        top: Box<Layout>,
        bottom: Box<Layout>,
        ratio: u16, // Percentage for top (0-100)
    },
}

impl Layout {
    /// Create single pane layout
    pub fn single(pane_id: PaneId) -> Self {
        Self::Single(pane_id)
    }

    /// Split horizontally (creates left | right)
    pub fn split_horizontal(self, new_pane_id: PaneId, ratio: u16) -> Self {
        Self::HSplit {
            left: Box::new(self),
            right: Box::new(Self::Single(new_pane_id)),
            ratio,
        }
    }

    /// Split vertically (creates top / bottom)
    pub fn split_vertical(self, new_pane_id: PaneId, ratio: u16) -> Self {
        Self::VSplit {
            top: Box::new(self),
            bottom: Box::new(Self::Single(new_pane_id)),
            ratio,
        }
    }

    /// Calculate rectangles for all panes in this layout
    pub fn calculate_rects(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        let mut result = Vec::new();
        self.calculate_rects_recursive(area, &mut result);
        result
    }

    fn calculate_rects_recursive(&self, area: Rect, result: &mut Vec<(PaneId, Rect)>) {
        match self {
            Layout::Single(id) => {
                result.push((*id, area));
            }
            Layout::HSplit { left, right, ratio } => {
                let chunks = RatatuiLayout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(*ratio),
                        Constraint::Percentage(100 - ratio),
                    ])
                    .split(area);

                left.calculate_rects_recursive(chunks[0], result);
                right.calculate_rects_recursive(chunks[1], result);
            }
            Layout::VSplit { top, bottom, ratio } => {
                let chunks = RatatuiLayout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(*ratio),
                        Constraint::Percentage(100 - ratio),
                    ])
                    .split(area);

                top.calculate_rects_recursive(chunks[0], result);
                bottom.calculate_rects_recursive(chunks[1], result);
            }
        }
    }

    /// Get all pane IDs in this layout
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_pane_ids(&mut ids);
        ids
    }

    fn collect_pane_ids(&self, ids: &mut Vec<PaneId>) {
        match self {
            Layout::Single(id) => ids.push(*id),
            Layout::HSplit { left, right, .. } => {
                left.collect_pane_ids(ids);
                right.collect_pane_ids(ids);
            }
            Layout::VSplit { top, bottom, .. } => {
                top.collect_pane_ids(ids);
                bottom.collect_pane_ids(ids);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_layout() {
        let id = PaneId::new();
        let layout = Layout::single(id);

        let area = Rect::new(0, 0, 100, 100);
        let rects = layout.calculate_rects(area);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, id);
        assert_eq!(rects[0].1, area);
    }

    #[test]
    fn test_hsplit_layout() {
        let id1 = PaneId::new();
        let id2 = PaneId::new();

        let layout = Layout::single(id1).split_horizontal(id2, 50);

        let area = Rect::new(0, 0, 100, 100);
        let rects = layout.calculate_rects(area);

        assert_eq!(rects.len(), 2);
    }
}
