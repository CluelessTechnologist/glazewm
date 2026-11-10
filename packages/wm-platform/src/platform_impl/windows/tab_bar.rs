use std::{cell::RefCell, collections::HashMap, sync::Once};

use tokio::sync::mpsc;
use windows::{
  core::{w, PCWSTR},
  Win32::{
    Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{
      BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
      CreateFontIndirectW, CreateSolidBrush, DeleteDC, DeleteObject,
      DrawTextW, EndPaint, FillRect, InvalidateRect, SelectObject,
      SetBkMode, SetTextColor, CLEARTYPE_QUALITY, DT_CENTER,
      DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HDC,
      HGDIOBJ, LOGFONTW, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
      CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
      LoadCursorW, RegisterClassW, SetWindowPos, ShowWindow, HWND_TOP,
      IDC_ARROW, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER,
      SWP_SHOWWINDOW, SW_HIDE, WM_ERASEBKGND, WM_LBUTTONDOWN,
      WM_MOUSEACTIVATE, WM_PAINT, WNDCLASSW, WS_EX_NOACTIVATE,
      WS_EX_TOOLWINDOW, WS_POPUP,
    },
  },
};

use crate::{
  Color, Dispatcher, Rect, Tab, TabBarEvent, TabBarId, TabBarStyle,
};

/// Window class name of tab bar windows.
const TAB_BAR_CLASS: PCWSTR = w!("GlazeWMTabBar");

/// Horizontal padding between a tab's edges and its title.
const TITLE_PADDING_PX: i32 = 6;

/// Ensures the window class is only registered once.
static REGISTER_CLASS: Once = Once::new();

thread_local! {
  /// State of all tab bars, keyed by window handle.
  ///
  /// Only accessed on the event loop thread, where the tab bar windows
  /// are created and their messages are processed.
  static TAB_BARS: RefCell<HashMap<isize, TabBarState>> =
    RefCell::new(HashMap::new());
}

/// State needed by the window procedure to paint and handle clicks.
struct TabBarState {
  id: TabBarId,
  event_tx: mpsc::UnboundedSender<TabBarEvent>,
  tabs: Vec<Tab>,
  style: Option<TabBarStyle>,
}

/// Platform-specific implementation of [`TabBar`].
pub(crate) struct TabBar {
  handle: isize,
  dispatcher: Dispatcher,
}

impl TabBar {
  /// Implements [`TabBar::new`].
  pub(crate) fn new(
    id: TabBarId,
    event_tx: mpsc::UnboundedSender<TabBarEvent>,
    dispatcher: &Dispatcher,
  ) -> crate::Result<Self> {
    let handle = dispatcher.dispatch_sync(move || {
      Self::create_window().inspect(|&handle| {
        TAB_BARS.with(|tab_bars| {
          tab_bars.borrow_mut().insert(
            handle,
            TabBarState {
              id,
              event_tx,
              tabs: Vec::new(),
              style: None,
            },
          );
        });
      })
    })??;

    Ok(Self {
      handle,
      dispatcher: dispatcher.clone(),
    })
  }

  /// Implements [`TabBar::show`].
  pub(crate) fn show(
    &self,
    rect: &Rect,
    tabs: &[Tab],
    style: &TabBarStyle,
  ) -> crate::Result<()> {
    let handle = self.handle;
    let rect = rect.clone();
    let tabs = tabs.to_vec();
    let style = style.clone();

    self.dispatcher.dispatch_sync(move || {
      let was_visible = TAB_BARS.with(|tab_bars| {
        tab_bars.borrow_mut().get_mut(&handle).map(|state| {
          let was_visible = state.style.is_some();
          state.tabs = tabs;
          state.style = Some(style);
          was_visible
        })
      });

      // Bring the bar to the top when it becomes visible. Afterwards,
      // keep its z-order so that it doesn't cover floating windows.
      let mut swp_flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
      if was_visible == Some(true) {
        swp_flags |= SWP_NOZORDER;
      }

      unsafe {
        SetWindowPos(
          HWND(handle),
          HWND_TOP,
          rect.x(),
          rect.y(),
          rect.width(),
          rect.height(),
          swp_flags,
        )?;

        InvalidateRect(HWND(handle), None, false);
      }

      Ok(())
    })?
  }

  /// Implements [`TabBar::hide`].
  pub(crate) fn hide(&self) -> crate::Result<()> {
    let handle = self.handle;

    self.dispatcher.dispatch_sync(move || {
      TAB_BARS.with(|tab_bars| {
        if let Some(state) = tab_bars.borrow_mut().get_mut(&handle) {
          state.style = None;
        }
      });

      unsafe { ShowWindow(HWND(handle), SW_HIDE) };
    })
  }

  /// Creates a hidden tab bar window on the current thread.
  ///
  /// Returns the handle of the created window.
  fn create_window() -> crate::Result<isize> {
    let instance = unsafe { GetModuleHandleW(None) }?;

    REGISTER_CLASS.call_once(|| {
      let wnd_class = WNDCLASSW {
        lpszClassName: TAB_BAR_CLASS,
        lpfnWndProc: Some(Self::window_proc),
        hInstance: instance.into(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }
          .unwrap_or_default(),
        ..Default::default()
      };

      unsafe { RegisterClassW(&raw const wnd_class) };
    });

    // `WS_EX_NOACTIVATE` prevents the bar from stealing focus when
    // clicked, and together with `WS_EX_TOOLWINDOW` excludes it from the
    // taskbar and from being managed by the WM.
    let handle = unsafe {
      CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        TAB_BAR_CLASS,
        w!("GlazeWM Tab Bar"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        instance,
        None,
      )
    };

    if handle.0 == 0 {
      return Err(crate::Error::Platform(
        "Creation of tab bar window failed.".to_string(),
      ));
    }

    Ok(handle.0)
  }

  /// Window procedure for tab bar windows.
  unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
  ) -> LRESULT {
    match msg {
      WM_PAINT => {
        Self::paint(hwnd);
        LRESULT(0)
      }
      // Painting covers the whole client area, so skip erasing it to
      // avoid flicker.
      WM_ERASEBKGND => LRESULT(1),
      #[allow(clippy::cast_possible_wrap)]
      WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
      WM_LBUTTONDOWN => {
        // Low-order word of `LPARAM` is the signed x-coordinate, so the
        // truncation and reinterpretation as `i16` are intended.
        #[allow(
          clippy::cast_possible_truncation,
          clippy::cast_possible_wrap,
          clippy::cast_sign_loss
        )]
        let x = i32::from((lparam.0 & 0xFFFF) as u16 as i16);
        Self::handle_click(hwnd, x);
        LRESULT(0)
      }
      _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
  }

  /// Emits a click event for the tab at the given x-coordinate.
  fn handle_click(hwnd: HWND, x: i32) {
    let mut client_rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &raw mut client_rect) }.is_err() {
      return;
    }

    TAB_BARS.with(|tab_bars| {
      let tab_bars = tab_bars.borrow();
      let Some(state) = tab_bars.get(&hwnd.0) else {
        return;
      };

      if let Some(tab_index) =
        tab_index_at(x, client_rect.right, state.tabs.len())
      {
        let _ = state.event_tx.send(TabBarEvent::TabClicked {
          tab_bar_id: state.id,
          tab_index,
        });
      }
    });
  }

  /// Paints the tabs into an offscreen buffer and copies it to the
  /// window.
  unsafe fn paint(hwnd: HWND) {
    let mut paint_struct = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &raw mut paint_struct);

    let mut client_rect = RECT::default();
    if GetClientRect(hwnd, &raw mut client_rect).is_ok() {
      let width = client_rect.right;
      let height = client_rect.bottom;

      let buffer_dc = CreateCompatibleDC(hdc);
      let buffer_bitmap = CreateCompatibleBitmap(hdc, width, height);
      let prev_bitmap = SelectObject(buffer_dc, HGDIOBJ(buffer_bitmap.0));

      TAB_BARS.with(|tab_bars| {
        if let Some(state) = tab_bars.borrow().get(&hwnd.0) {
          if let Some(style) = &state.style {
            draw_tabs(buffer_dc, width, height, &state.tabs, style);
          }
        }
      });

      let _ = BitBlt(hdc, 0, 0, width, height, buffer_dc, 0, 0, SRCCOPY);

      SelectObject(buffer_dc, prev_bitmap);
      DeleteObject(HGDIOBJ(buffer_bitmap.0));
      DeleteDC(buffer_dc);
    }

    EndPaint(hwnd, &raw const paint_struct);
  }
}

impl Drop for TabBar {
  fn drop(&mut self) {
    let handle = self.handle;

    let res = self.dispatcher.dispatch_async(move || {
      TAB_BARS.with(|tab_bars| tab_bars.borrow_mut().remove(&handle));
      let _ = unsafe { DestroyWindow(HWND(handle)) };
    });

    if let Err(err) = res {
      tracing::warn!("Failed to destroy tab bar window: {err}");
    }
  }
}

/// Gets the index of the tab at the given x-coordinate, where `count`
/// tabs are evenly spread over `width`.
fn tab_index_at(x: i32, width: i32, count: usize) -> Option<usize> {
  if count == 0 || width <= 0 || x < 0 || x >= width {
    return None;
  }

  let count = i64::try_from(count).ok()?;
  let index = i64::from(x) * count / i64::from(width);

  usize::try_from(index).ok()
}

/// Gets the left and right edge of the tab at `index`, where `count`
/// tabs are evenly spread over `width`.
///
/// Edges are rounded so that the tabs cover the full width without gaps.
fn tab_edges(index: usize, width: i32, count: usize) -> (i32, i32) {
  let edge = |i: usize| -> i32 {
    let i = i64::try_from(i).unwrap_or(i64::MAX);
    let count = i64::try_from(count.max(1)).unwrap_or(i64::MAX);

    // Result is within `0..=width` since `i <= count`.
    i32::try_from(i64::from(width) * i.min(count) / count).unwrap_or(width)
  };

  (edge(index), edge(index + 1))
}

/// Draws all tabs onto the given device context.
unsafe fn draw_tabs(
  hdc: HDC,
  width: i32,
  height: i32,
  tabs: &[Tab],
  style: &TabBarStyle,
) {
  let font = create_font(style);
  let prev_font = SelectObject(hdc, HGDIOBJ(font.0));
  SetBkMode(hdc, TRANSPARENT);

  // Fill with the unfocused background in case there are no tabs.
  fill_rect(
    hdc,
    &RECT {
      left: 0,
      top: 0,
      right: width,
      bottom: height,
    },
    &style.unfocused.background,
  );

  for (index, tab) in tabs.iter().enumerate() {
    let colors = style.colors(tab.state);
    let (left, right) = tab_edges(index, width, tabs.len());

    let tab_rect = RECT {
      left,
      top: 0,
      right,
      bottom: height,
    };

    // Draw the border by filling the full tab, then the background inset
    // by 1px.
    fill_rect(hdc, &tab_rect, &colors.border);
    fill_rect(
      hdc,
      &RECT {
        left: tab_rect.left + 1,
        top: tab_rect.top + 1,
        right: tab_rect.right - 1,
        bottom: tab_rect.bottom - 1,
      },
      &colors.background,
    );

    let mut text_rect = RECT {
      left: tab_rect.left + TITLE_PADDING_PX,
      top: tab_rect.top,
      right: tab_rect.right - TITLE_PADDING_PX,
      bottom: tab_rect.bottom,
    };

    let mut title = tab.title.encode_utf16().collect::<Vec<_>>();
    SetTextColor(hdc, COLORREF(colors.text.to_bgr()));
    DrawTextW(
      hdc,
      &mut title,
      &raw mut text_rect,
      DT_CENTER
        | DT_VCENTER
        | DT_SINGLELINE
        | DT_END_ELLIPSIS
        | DT_NOPREFIX,
    );
  }

  SelectObject(hdc, prev_font);
  DeleteObject(HGDIOBJ(font.0));
}

/// Fills a rect with a solid color.
unsafe fn fill_rect(hdc: HDC, rect: &RECT, color: &Color) {
  let brush = CreateSolidBrush(COLORREF(color.to_bgr()));
  FillRect(hdc, rect, brush);
  DeleteObject(HGDIOBJ(brush.0));
}

/// Creates the font for tab titles.
///
/// The caller is responsible for deleting the font.
unsafe fn create_font(
  style: &TabBarStyle,
) -> windows::Win32::Graphics::Gdi::HFONT {
  let mut log_font = LOGFONTW {
    // Negative height selects by character height rather than cell
    // height, which matches how font sizes are usually specified.
    lfHeight: -style.font_size,
    lfWeight: 400,
    lfQuality: CLEARTYPE_QUALITY,
    ..Default::default()
  };

  // Copy the font family, leaving room for the null terminator.
  for (dest, src) in log_font
    .lfFaceName
    .iter_mut()
    .zip(style.font_family.encode_utf16().take(31))
  {
    *dest = src;
  }

  CreateFontIndirectW(&raw const log_font)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tab_index_maps_evenly() {
    assert_eq!(tab_index_at(0, 300, 3), Some(0));
    assert_eq!(tab_index_at(99, 300, 3), Some(0));
    assert_eq!(tab_index_at(100, 300, 3), Some(1));
    assert_eq!(tab_index_at(299, 300, 3), Some(2));
  }

  #[test]
  fn tab_index_out_of_bounds() {
    assert_eq!(tab_index_at(-1, 300, 3), None);
    assert_eq!(tab_index_at(300, 300, 3), None);
    assert_eq!(tab_index_at(10, 300, 0), None);
  }

  #[test]
  fn tab_edges_cover_full_width() {
    assert_eq!(tab_edges(0, 100, 3), (0, 33));
    assert_eq!(tab_edges(1, 100, 3), (33, 66));
    assert_eq!(tab_edges(2, 100, 3), (66, 100));
  }
}
