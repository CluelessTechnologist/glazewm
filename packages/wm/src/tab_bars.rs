use std::collections::{hash_map::Entry, HashMap};

use tokio::sync::mpsc;
use uuid::Uuid;
use wm_platform::{
  Dispatcher, Rect, Tab, TabBar, TabBarEvent, TabBarId, TabBarStyle,
};

/// Desired state of a tab bar that should be visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabBarContent {
  pub rect: Rect,
  pub tabs: Vec<Tab>,
  pub style: TabBarStyle,
}

/// A native tab bar along with what was last drawn to it.
struct ManagedTabBar {
  tab_bar: TabBar,

  /// Content that is currently shown. `None` if hidden.
  shown: Option<TabBarContent>,
}

/// Native tab bars of containers with the tabbed layout.
pub struct TabBars {
  /// Tab bars keyed by the ID of the container they belong to.
  tab_bars: HashMap<Uuid, ManagedTabBar>,

  /// Sender for interactions with any of the tab bars.
  event_tx: mpsc::UnboundedSender<TabBarEvent>,
}

impl TabBars {
  /// Creates an empty set of tab bars, where interactions are sent
  /// through `event_tx`.
  pub fn new(event_tx: mpsc::UnboundedSender<TabBarEvent>) -> Self {
    Self {
      tab_bars: HashMap::new(),
      event_tx,
    }
  }

  /// Gets the ID of the container that the given tab bar belongs to.
  pub fn container_id(&self, tab_bar_id: TabBarId) -> Option<Uuid> {
    self
      .tab_bars
      .iter()
      .find(|(_, managed)| managed.tab_bar.id() == tab_bar_id)
      .map(|(container_id, _)| *container_id)
  }

  /// Updates the tab bars to match the desired state.
  ///
  /// `desired` contains an entry for every container with the tabbed
  /// layout, with `None` if its tab bar should be hidden. Tab bars of
  /// containers not in `desired` are destroyed. Native tab bars are only
  /// updated if their content has changed.
  pub fn sync(
    &mut self,
    desired: HashMap<Uuid, Option<TabBarContent>>,
    dispatcher: &Dispatcher,
  ) -> anyhow::Result<()> {
    self
      .tab_bars
      .retain(|container_id, _| desired.contains_key(container_id));

    for (container_id, content) in desired {
      let managed = match self.tab_bars.entry(container_id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
          // No need to create a tab bar until it's shown.
          if content.is_none() {
            continue;
          }

          entry.insert(ManagedTabBar {
            tab_bar: TabBar::new(self.event_tx.clone(), dispatcher)?,
            shown: None,
          })
        }
      };

      match content {
        Some(content) => {
          if managed.shown.as_ref() != Some(&content) {
            managed.tab_bar.show(
              &content.rect,
              &content.tabs,
              &content.style,
            )?;
            managed.shown = Some(content);
          }
        }
        None => {
          if managed.shown.take().is_some() {
            managed.tab_bar.hide()?;
          }
        }
      }
    }

    Ok(())
  }

  /// Destroys all tab bars.
  pub fn clear(&mut self) {
    self.tab_bars.clear();
  }
}
