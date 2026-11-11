use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{platform_impl, Color, Dispatcher, Rect};

/// Unique identifier of a [`TabBar`].
pub type TabBarId = u64;

/// Source of unique IDs for created tab bars.
static NEXT_TAB_BAR_ID: AtomicU64 = AtomicU64::new(0);

/// Visual state of a tab within a [`TabBar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabState {
  /// Active tab of the container that has focus.
  Focused,

  /// Active tab of a container that doesn't have focus.
  FocusedInactive,

  /// Tab that isn't active.
  Unfocused,
}

/// A single tab shown in a [`TabBar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
  /// Text shown in the tab.
  pub title: String,

  /// Visual state of the tab.
  pub state: TabState,
}

/// Colors used to draw a tab in a given [`TabState`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabColors {
  pub background: Color,
  pub border: Color,
  pub text: Color,
}

/// Horizontal alignment of the titles within their tabs.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TabTextAlign {
  /// Like Sway/i3's default `title_align left`.
  #[default]
  Left,
  Center,
}

/// Appearance of a [`TabBar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabBarStyle {
  /// Font family of the tab titles.
  pub font_family: String,

  /// Font size of the tab titles in physical pixels.
  pub font_size: i32,

  /// Horizontal alignment of the tab titles.
  pub text_align: TabTextAlign,

  /// Colors for [`TabState::Focused`].
  pub focused: TabColors,

  /// Colors for [`TabState::FocusedInactive`].
  pub focused_inactive: TabColors,

  /// Colors for [`TabState::Unfocused`].
  pub unfocused: TabColors,
}

impl TabBarStyle {
  /// Gets the colors to draw a tab with the given state.
  #[must_use]
  pub fn colors(&self, state: TabState) -> &TabColors {
    match state {
      TabState::Focused => &self.focused,
      TabState::FocusedInactive => &self.focused_inactive,
      TabState::Unfocused => &self.unfocused,
    }
  }
}

/// Event emitted by user interaction with a [`TabBar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabBarEvent {
  /// A tab was clicked with the left mouse button.
  TabClicked {
    tab_bar_id: TabBarId,
    tab_index: usize,
  },
}

/// A bar of clickable tabs drawn by the WM above a container.
///
/// The bar is hidden until [`TabBar::show`] is called, and is destroyed
/// when dropped.
///
/// # Platform-specific
///
/// - **macOS**: Not yet implemented. The bar is never shown and no events
///   are emitted.
pub struct TabBar {
  id: TabBarId,

  /// Inner platform-specific tab bar.
  inner: platform_impl::TabBar,
}

impl TabBar {
  /// Creates a new hidden [`TabBar`].
  ///
  /// Clicks on the bar are sent through `event_tx`.
  pub fn new(
    event_tx: mpsc::UnboundedSender<TabBarEvent>,
    dispatcher: &Dispatcher,
  ) -> crate::Result<Self> {
    let id = NEXT_TAB_BAR_ID.fetch_add(1, Ordering::Relaxed);
    let inner = platform_impl::TabBar::new(id, event_tx, dispatcher)?;

    Ok(Self { id, inner })
  }

  /// Gets the unique ID of the tab bar.
  #[must_use]
  pub fn id(&self) -> TabBarId {
    self.id
  }

  /// Shows the tab bar at the given rect and redraws it with the given
  /// tabs, which are evenly sized from left to right.
  pub fn show(
    &self,
    rect: &Rect,
    tabs: &[Tab],
    style: &TabBarStyle,
  ) -> crate::Result<()> {
    self.inner.show(rect, tabs, style)
  }

  /// Hides the tab bar.
  pub fn hide(&self) -> crate::Result<()> {
    self.inner.hide()
  }
}
