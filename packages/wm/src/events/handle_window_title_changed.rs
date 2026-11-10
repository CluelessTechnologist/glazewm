use tracing::info;
use wm_common::{try_warn, WindowRuleEvent};
use wm_platform::NativeWindow;

use crate::{
  commands::window::run_window_rules, traits::WindowGetters,
  user_config::UserConfig, wm_state::WmState,
};

pub fn handle_window_title_changed(
  native_window: &NativeWindow,
  state: &mut WmState,
  config: &mut UserConfig,
) -> anyhow::Result<()> {
  let found_window = state.window_from_native(native_window);

  if let Some(window) = found_window {
    info!("Window title changed: {window}");

    let title = try_warn!(window.native().title());

    window.update_native_properties(|properties| {
      properties.title = title;
    });

    // Tab titles are derived from window titles.
    state.pending_sync.queue_tab_bar_update();

    // Run window rules for title change events.
    run_window_rules(
      window,
      &WindowRuleEvent::TitleChange,
      state,
      config,
    )?;
  }

  Ok(())
}
