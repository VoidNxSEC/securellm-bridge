// ! Multiplex system for Zellij-style TUI

mod layout;
mod pane;
mod tabs;

pub use layout::Layout;
pub use pane::{Pane, PaneId};
pub use tabs::TabBar;
