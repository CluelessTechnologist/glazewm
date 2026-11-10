use ambassador::delegatable_trait;
use wm_platform::Rect;

#[delegatable_trait]
pub trait PositionGetters {
  fn to_rect(&self) -> anyhow::Result<Rect>;
}

/// Gets the rect of the tab bar in a container with the tabbed layout.
///
/// The bar spans the top of the container, and its height is clamped to
/// the container's height.
#[must_use]
pub fn tab_bar_rect(parent_rect: &Rect, tab_bar_height: i32) -> Rect {
  Rect::from_xy(
    parent_rect.x(),
    parent_rect.y(),
    parent_rect.width(),
    tab_bar_height.clamp(0, parent_rect.height().max(0)),
  )
}

/// Gets the rect of a child in a container with the tabbed layout.
///
/// Every child takes up the full container below the tab bar.
#[must_use]
pub fn tabbed_child_rect(parent_rect: &Rect, tab_bar_height: i32) -> Rect {
  let bar_height = tab_bar_rect(parent_rect, tab_bar_height).height();

  Rect::from_xy(
    parent_rect.x(),
    parent_rect.y() + bar_height,
    parent_rect.width(),
    parent_rect.height() - bar_height,
  )
}

/// Implements the `PositionGetters` trait for tiling containers that can
/// be resized. This is used by `SplitContainer` and `TilingWindow`.
///
/// Expects that the struct has a wrapping `RefCell` containing a struct
/// with an `id` and a `parent` field.
#[macro_export]
macro_rules! impl_position_getters_as_resizable {
  ($struct_name:ident) => {
    impl PositionGetters for $struct_name {
      fn to_rect(&self) -> anyhow::Result<Rect> {
        let parent = self
          .parent()
          .and_then(|parent| parent.as_direction_container().ok())
          .context("Parent does not have a tiling direction.")?;

        let parent_rect = parent.to_rect()?;

        if parent.tiling_layout() == wm_common::TilingLayout::Tabbed {
          return Ok($crate::traits::tabbed_child_rect(
            &parent_rect,
            self.tab_bar_height()?,
          ));
        }

        let (horizontal_gap, vertical_gap) = self.inner_gaps()?;
        let inner_gap = match parent.tiling_direction() {
          TilingDirection::Vertical => vertical_gap,
          TilingDirection::Horizontal => horizontal_gap,
        };

        #[allow(
          clippy::cast_precision_loss,
          clippy::cast_possible_truncation,
          clippy::cast_possible_wrap
        )]
        let (width, height) = match parent.tiling_direction() {
          TilingDirection::Vertical => {
            let available_height = parent_rect.height()
              - inner_gap * self.tiling_siblings().count() as i32;

            let height =
              (self.tiling_size() * available_height as f32) as i32;

            (parent_rect.width(), height)
          }
          TilingDirection::Horizontal => {
            let available_width = parent_rect.width()
              - inner_gap * self.tiling_siblings().count() as i32;

            let width =
              (available_width as f32 * self.tiling_size()).round() as i32;

            (width, parent_rect.height())
          }
        };

        let (x, y) = {
          let mut prev_siblings = self
            .prev_siblings()
            .filter_map(|sibling| sibling.as_tiling_container().ok());

          match prev_siblings.next() {
            None => (parent_rect.x(), parent_rect.y()),
            Some(sibling) => {
              let sibling_rect = sibling.to_rect()?;

              match parent.tiling_direction() {
                TilingDirection::Vertical => (
                  parent_rect.x(),
                  sibling_rect.y() + sibling_rect.height() + inner_gap,
                ),
                TilingDirection::Horizontal => (
                  sibling_rect.x() + sibling_rect.width() + inner_gap,
                  parent_rect.y(),
                ),
              }
            }
          }
        };

        Ok(Rect::from_xy(x, y, width, height))
      }
    }
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tab_bar_spans_top_of_parent() {
    let rect = tab_bar_rect(&Rect::from_xy(10, 20, 800, 600), 24);
    assert_eq!(
      (rect.x(), rect.y(), rect.width(), rect.height()),
      (10, 20, 800, 24)
    );
  }

  #[test]
  fn tabbed_child_fills_parent_below_bar() {
    let rect = tabbed_child_rect(&Rect::from_xy(10, 20, 800, 600), 24);
    assert_eq!(
      (rect.x(), rect.y(), rect.width(), rect.height()),
      (10, 44, 800, 576)
    );
  }

  #[test]
  fn tab_bar_height_is_clamped_to_parent() {
    let parent = Rect::from_xy(0, 0, 800, 10);
    assert_eq!(tab_bar_rect(&parent, 24).height(), 10);
    assert_eq!(tabbed_child_rect(&parent, 24).height(), 0);
    assert_eq!(tab_bar_rect(&parent, -5).height(), 0);
  }
}
