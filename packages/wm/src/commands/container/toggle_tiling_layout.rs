use anyhow::Context;
use wm_common::{TilingDirection, TilingLayout, WmEvent};

use crate::{
  models::{Container, DirectionContainer},
  traits::{CommonGetters, TilingDirectionGetters},
  wm_state::WmState,
};

/// Toggles the layout of the direction container (split container or
/// workspace) that the given container is in between tiles and tabbed.
pub fn toggle_tiling_layout(
  container: &Container,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Some(direction_container) = layout_target(container)? else {
    return Ok(());
  };

  let tiling_layout = direction_container.tiling_layout().toggled();
  update_tiling_layout(&direction_container, tiling_layout, state)
}

/// Sets the layout of the direction container (split container or
/// workspace) that the given container is in.
pub fn set_tiling_layout(
  container: &Container,
  tiling_layout: &TilingLayout,
  state: &mut WmState,
) -> anyhow::Result<()> {
  let Some(direction_container) = layout_target(container)? else {
    return Ok(());
  };

  if direction_container.tiling_layout() == *tiling_layout {
    return Ok(());
  }

  update_tiling_layout(&direction_container, tiling_layout.clone(), state)
}

/// Changes the layout of a direction container and queues a redraw of the
/// container and all of its descendants.
///
/// Tabbed containers are always horizontal, such that focusing or moving
/// left/right switches between tabs.
pub fn update_tiling_layout(
  direction_container: &DirectionContainer,
  tiling_layout: TilingLayout,
  state: &mut WmState,
) -> anyhow::Result<()> {
  if tiling_layout == TilingLayout::Tabbed {
    direction_container.set_tiling_direction(TilingDirection::Horizontal);
  }

  direction_container.set_tiling_layout(tiling_layout.clone());

  state
    .pending_sync
    .queue_container_to_redraw(direction_container.clone());

  state.emit_event(WmEvent::TilingLayoutChanged {
    direction_container: direction_container.to_dto()?,
    new_tiling_layout: tiling_layout,
  });

  Ok(())
}

/// Gets the direction container whose layout should be changed.
///
/// Only tiling windows and workspaces have a target, which is the nearest
/// direction container.
fn layout_target(
  container: &Container,
) -> anyhow::Result<Option<DirectionContainer>> {
  match container {
    Container::TilingWindow(_) | Container::Workspace(_) => Ok(Some(
      container
        .direction_container()
        .context("No direction container.")?,
    )),
    _ => Ok(None),
  }
}
