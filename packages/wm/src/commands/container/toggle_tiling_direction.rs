use anyhow::Context;
use wm_common::{TilingDirection, TilingLayout, WmEvent};

use super::{
  flatten_split_container, update_tiling_layout, wrap_in_split_container,
};
use crate::{
  models::{Container, DirectionContainer, SplitContainer, TilingWindow},
  traits::{CommonGetters, TilingDirectionGetters},
  user_config::UserConfig,
  wm_state::WmState,
};

pub fn toggle_tiling_direction(
  container: Container,
  state: &mut WmState,
  config: &UserConfig,
) -> anyhow::Result<()> {
  let direction_container = match container {
    Container::TilingWindow(tiling_window) => {
      toggle_window_direction(tiling_window, state, config)
    }
    Container::Workspace(workspace) => {
      let workspace = DirectionContainer::from(workspace);
      invert_tiling_direction(&workspace, state)?;

      Ok(workspace)
    }
    // Can only toggle tiling direction from a tiling window or workspace.
    _ => return Ok(()),
  }?;

  state.emit_event(WmEvent::TilingDirectionChanged {
    direction_container: direction_container.to_dto()?,
    new_tiling_direction: direction_container.tiling_direction(),
  });

  Ok(())
}

fn toggle_window_direction(
  tiling_window: TilingWindow,
  state: &mut WmState,
  config: &UserConfig,
) -> anyhow::Result<DirectionContainer> {
  let parent = tiling_window
    .direction_container()
    .context("No direction container.")?;

  // If the window is an only child, then either change the tiling
  // direction of its parent workspace or flatten its parent split
  // container.
  if tiling_window.tiling_siblings().count() == 0 {
    return match parent {
      DirectionContainer::Workspace(workspace) => {
        let workspace = DirectionContainer::from(workspace);
        invert_tiling_direction(&workspace, state)?;

        Ok(workspace)
      }
      DirectionContainer::Split(split_container) => {
        flatten_split_container(split_container.clone())?;

        tiling_window
          .direction_container()
          .context("No direction container.")
      }
    };
  }

  // Create a new split container to wrap the window.
  let split_container = SplitContainer::new(
    parent.tiling_direction().inverse(),
    config.value.gaps.clone(),
  );

  wrap_in_split_container(
    &split_container,
    &parent.into(),
    &[tiling_window.into()],
  )?;

  Ok(split_container.into())
}

/// Inverts the tiling direction of a direction container.
///
/// Tabbed containers are always horizontal, so they are instead switched
/// to the tiles layout (similar to `layout toggle split` in Sway).
fn invert_tiling_direction(
  direction_container: &DirectionContainer,
  state: &mut WmState,
) -> anyhow::Result<()> {
  if direction_container.tiling_layout() == TilingLayout::Tabbed {
    return update_tiling_layout(
      direction_container,
      TilingLayout::Tiles,
      state,
    );
  }

  direction_container.set_tiling_direction(
    direction_container.tiling_direction().inverse(),
  );

  Ok(())
}

pub fn set_tiling_direction(
  container: Container,
  state: &mut WmState,
  config: &UserConfig,
  tiling_direction: &TilingDirection,
) -> anyhow::Result<()> {
  let direction_container = container
    .direction_container()
    .context("No direction container.")?;

  if direction_container.tiling_direction() == *tiling_direction {
    Ok(())
  } else {
    toggle_tiling_direction(container, state, config)
  }
}
