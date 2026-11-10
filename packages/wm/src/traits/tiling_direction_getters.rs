use ambassador::delegatable_trait;
use wm_common::{TilingDirection, TilingLayout};
use wm_platform::Direction;

use super::CommonGetters;
use crate::models::{TilingContainer, TilingWindow};

#[delegatable_trait]
pub trait TilingDirectionGetters: CommonGetters {
  fn tiling_direction(&self) -> TilingDirection;

  fn set_tiling_direction(&self, tiling_direction: TilingDirection);

  fn tiling_layout(&self) -> TilingLayout;

  fn set_tiling_layout(&self, tiling_layout: TilingLayout);

  /// Gets the active tab of a container with the tabbed layout, which is
  /// its most recently focused tiling child.
  ///
  /// Returns `None` if the container doesn't use the tabbed layout or has
  /// no tiling children.
  fn active_tab(&self) -> Option<TilingContainer> {
    if self.tiling_layout() != TilingLayout::Tabbed {
      return None;
    }

    self
      .child_focus_order()
      .find_map(|child| child.as_tiling_container().ok())
  }

  /// Traverses down a container in search of a descendant in the given
  /// direction. For example, for `Direction::Right`, get the right-most
  /// container.
  ///
  /// Any non-tiling containers are ignored.
  fn descendant_in_direction(
    &self,
    direction: &Direction,
  ) -> Option<TilingWindow> {
    let child = self.child_in_direction(direction)?;

    // Traverse further down if the child is a split container.
    match child {
      TilingContainer::Split(split_child) => {
        split_child.descendant_in_direction(direction)
      }
      TilingContainer::TilingWindow(window) => Some(window),
    }
  }

  fn child_in_direction(
    &self,
    direction: &Direction,
  ) -> Option<TilingContainer> {
    // Only the active tab is visible in a tabbed container, so traverse
    // into it regardless of direction.
    if self.tiling_layout() == TilingLayout::Tabbed {
      return self.active_tab();
    }

    // When the tiling direction is the inverse of the direction, return
    // the last focused tiling child.
    if self.tiling_direction()
      != TilingDirection::from_direction(direction)
    {
      return self
        .child_focus_order()
        .find_map(|c| c.as_tiling_container().ok());
    }

    match direction {
      Direction::Up | Direction::Left => self.tiling_children().next(),
      _ => self.tiling_children().last(),
    }
  }
}

/// Implements the `TilingDirectionGetters` trait for a given struct.
///
/// Expects that the struct has a wrapping `RefCell` containing a struct
/// with `tiling_direction` and `tiling_layout` fields.
#[macro_export]
macro_rules! impl_tiling_direction_getters {
  ($struct_name:ident) => {
    impl TilingDirectionGetters for $struct_name {
      fn tiling_direction(&self) -> TilingDirection {
        self.0.borrow().tiling_direction.clone()
      }

      fn set_tiling_direction(&self, tiling_direction: TilingDirection) {
        self.0.borrow_mut().tiling_direction = tiling_direction;
      }

      fn tiling_layout(&self) -> wm_common::TilingLayout {
        self.0.borrow().tiling_layout.clone()
      }

      fn set_tiling_layout(&self, tiling_layout: wm_common::TilingLayout) {
        self.0.borrow_mut().tiling_layout = tiling_layout;
      }
    }
  };
}

#[cfg(test)]
mod tests {
  use wm_common::TilingLayout;

  use super::TilingDirectionGetters;
  use crate::{
    commands::container::set_focused_descendant,
    models::{NonTilingWindow, SplitContainer, TilingWindow, Workspace},
    traits::CommonGetters,
  };

  /// Creates a workspace with two tiling windows and a floating window.
  fn workspace_with_floating(
  ) -> (Workspace, TilingWindow, TilingWindow, NonTilingWindow) {
    let window_1 = TilingWindow::mock().call();
    let window_2 = TilingWindow::mock().call();
    let floating = NonTilingWindow::mock().call();

    let workspace = Workspace::mock()
      .tiling_containers(vec![
        window_1.clone().into(),
        window_2.clone().into(),
      ])
      .non_tiling_windows(vec![floating.clone()])
      .call();

    (workspace, window_1, window_2, floating)
  }

  #[test]
  fn child_focus_order_yields_each_child_once() {
    let (workspace, window_1, window_2, floating) =
      workspace_with_floating();

    set_focused_descendant(&window_1.clone().into(), None);
    set_focused_descendant(&floating.clone().into(), None);

    let focus_order = workspace
      .child_focus_order()
      .map(|child| child.id())
      .collect::<Vec<_>>();

    assert_eq!(focus_order.len(), 3);
    assert_eq!(focus_order[0], floating.id());
    assert_eq!(focus_order[1], window_1.id());
    assert!(focus_order.contains(&window_2.id()));
  }

  #[test]
  fn active_tab_skips_non_tiling_children() {
    let (workspace, _, window_2, floating) = workspace_with_floating();
    workspace.set_tiling_layout(TilingLayout::Tabbed);

    set_focused_descendant(&window_2.clone().into(), None);
    set_focused_descendant(&floating.into(), None);

    assert_eq!(
      workspace.active_tab().map(|tab| tab.id()),
      Some(window_2.id())
    );
  }

  #[test]
  fn tiles_layout_has_no_active_tab() {
    let (workspace, window_1, _, _) = workspace_with_floating();
    set_focused_descendant(&window_1.into(), None);

    assert!(workspace.active_tab().is_none());
  }

  #[test]
  fn only_active_tab_is_visible() {
    let window_1 = TilingWindow::mock().call();
    let window_2 = TilingWindow::mock().call();
    let window_3 = TilingWindow::mock().call();
    let floating = NonTilingWindow::mock().call();

    let split = SplitContainer::mock()
      .tiling_containers(vec![
        window_2.clone().into(),
        window_3.clone().into(),
      ])
      .call();

    let workspace = Workspace::mock()
      .tiling_containers(vec![window_1.clone().into(), split.into()])
      .non_tiling_windows(vec![floating.clone()])
      .call();

    workspace.set_tiling_layout(TilingLayout::Tabbed);

    set_focused_descendant(&window_1.clone().into(), None);
    assert!(!window_1.is_in_inactive_tab());
    assert!(window_2.is_in_inactive_tab());
    assert!(window_3.is_in_inactive_tab());
    assert!(!floating.is_in_inactive_tab());

    // Both windows in the active tab's split container are visible.
    set_focused_descendant(&window_3.clone().into(), None);
    assert!(window_1.is_in_inactive_tab());
    assert!(!window_2.is_in_inactive_tab());
    assert!(!window_3.is_in_inactive_tab());
  }
}
