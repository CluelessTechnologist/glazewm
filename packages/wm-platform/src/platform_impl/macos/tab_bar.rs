use tokio::sync::mpsc;

use crate::{Dispatcher, Rect, Tab, TabBarEvent, TabBarId, TabBarStyle};

/// Platform-specific implementation of [`TabBar`].
///
/// Drawing tab bars is not yet implemented on macOS, so this is a no-op.
pub(crate) struct TabBar;

impl TabBar {
  /// Implements [`TabBar::new`].
  #[allow(clippy::unnecessary_wraps)]
  pub(crate) fn new(
    _id: TabBarId,
    _event_tx: mpsc::UnboundedSender<TabBarEvent>,
    _dispatcher: &Dispatcher,
  ) -> crate::Result<Self> {
    Ok(Self)
  }

  /// Implements [`TabBar::show`].
  #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
  pub(crate) fn show(
    &self,
    _rect: &Rect,
    _tabs: &[Tab],
    _style: &TabBarStyle,
  ) -> crate::Result<()> {
    Ok(())
  }

  /// Implements [`TabBar::hide`].
  #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
  pub(crate) fn hide(&self) -> crate::Result<()> {
    Ok(())
  }
}
