use wm_common::TilingLayout;

use super::flatten_split_container;
use crate::{
  models::Container,
  traits::{CommonGetters, TilingDirectionGetters},
};

/// Flattens any redundant split containers at the top-level of the given
/// parent container.
///
/// For example:
/// ```ignore,compile_fail
/// H[1 H[V[2, 3]]] -> H[1, 2, 3]
/// H[1 H[2, 3]] -> H[1, 2, 3]
/// H[V[1]] -> V[1]
/// H[T[1, 2]] -> T[1, 2]
/// ```
///
/// Tabbed containers are never merged into their parent, and the
/// children of a tabbed container are left as-is, since each child is a
/// separate tab.
pub fn flatten_child_split_containers(
  parent: &Container,
) -> anyhow::Result<()> {
  if let Ok(parent) = parent.as_direction_container() {
    if parent.tiling_layout() == TilingLayout::Tabbed {
      return Ok(());
    }

    // Get children that are either tiling windows or split containers.
    let tiling_children = parent
      .children()
      .into_iter()
      .filter(|child| child.is_tiling_window() || child.is_split())
      .collect::<Vec<_>>();

    if tiling_children.len() == 1 {
      // Handle case where the parent is a split container and has a
      // single split container child.
      if let Some(split_child) = tiling_children[0].as_split() {
        // The parent takes over the layout of a tabbed child. Otherwise,
        // the child has the inverse tiling direction of the parent.
        let (tiling_direction, tiling_layout) =
          if split_child.tiling_layout() == TilingLayout::Tabbed {
            (split_child.tiling_direction(), TilingLayout::Tabbed)
          } else {
            (parent.tiling_direction().inverse(), parent.tiling_layout())
          };

        flatten_split_container(split_child.clone())?;
        parent.set_tiling_direction(tiling_direction);
        parent.set_tiling_layout(tiling_layout);
      }
    } else {
      let split_children = tiling_children
        .into_iter()
        .filter_map(|child| child.as_split().cloned())
        .collect::<Vec<_>>();

      for split_child in split_children.iter().filter(|split_child| {
        split_child.tiling_direction() == parent.tiling_direction()
          && split_child.tiling_layout() != TilingLayout::Tabbed
      }) {
        // Additionally flatten redundant top-level split containers in
        // the child.
        if split_child.child_count() == 1 {
          if let Some(split_grandchild) =
            split_child.children()[0].as_split().filter(|grandchild| {
              grandchild.tiling_layout() != TilingLayout::Tabbed
            })
          {
            flatten_split_container(split_grandchild.clone())?;
          }
        }

        flatten_split_container(split_child.clone())?;
      }
    }
  }

  Ok(())
}
