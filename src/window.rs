use crate::monitor::get_primary_monitor_logical_size;
use crate::{ICON_DATA, WINDOW_LOGICAL_SIZE};

use anyhow::{anyhow, Context, Result};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::EventLoop,
    platform::windows::WindowBuilderExtWindows,
    platform::windows::WindowExtWindows,
    window::{Icon, Window, WindowBuilder},
};
use windows::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR},
    Graphics::Gdi::UpdateWindow,
    UI::WindowsAndMessaging::{
        SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE,
        LAYERED_WINDOW_ATTRIBUTES_FLAGS, SW_SHOW, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    },
};

fn get_window_center_position(scale: f64) -> Result<(f64, f64)> {
    let (monitor_logical_width, monitor_logical_height) =
        get_primary_monitor_logical_size(scale)
            .map_err(|e| anyhow!("Failed to get primary monitor size- {e}"))?;
    let pos_logical_x = (monitor_logical_width - WINDOW_LOGICAL_SIZE) / 2.0;
    let pos_logical_y = (monitor_logical_height - WINDOW_LOGICAL_SIZE) / 2.0;
    Ok((pos_logical_x, pos_logical_y))
}

pub fn create_window(event_loop: &EventLoop<()>, scale: f64) -> Result<Window> {
    let (pos_logical_x, pos_logical_y) = get_window_center_position(scale)
        .map_err(|e| anyhow!("Failed to get window center position - {e}"))?;

    let window = WindowBuilder::new()
        .with_title("CapsLock")
        .with_window_icon(Some(
            load_icon(ICON_DATA).map_err(|e| anyhow!("Failed to load icon - {e}"))?,
        ))
        .with_inner_size(LogicalSize::new(WINDOW_LOGICAL_SIZE, WINDOW_LOGICAL_SIZE))
        .with_position(LogicalPosition::new(pos_logical_x, pos_logical_y))
        .with_skip_taskbar(!cfg!(debug_assertions))
        .with_undecorated_shadow(cfg!(debug_assertions))
        .with_content_protection(!cfg!(debug_assertions))
        .with_always_on_top(true)
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_focused(false)
        .build(event_loop)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    set_mouse_penetrable_layered_window(window.hwnd())
        .map_err(|e| anyhow!("Failed to set mouse penetrable layered window - {e}"))?;

    Ok(window)
}

fn set_mouse_penetrable_layered_window(hwnd: isize) -> Result<()> {
    unsafe {
        let hwnd = HWND(hwnd as _);
        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT;
        SetLastError(WIN32_ERROR(0));
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style.0 as isize);
        if GetLastError().0 == 0 {
            SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0), /* crKey */
                255,                                     /* Alpha: 0 ~ 255 */
                LAYERED_WINDOW_ATTRIBUTES_FLAGS(0x00000002), /* LWA_ALPHA: 0x00000002(窗口透明), LWA_COLORKEY: 0x0x00000001(指定crKey颜色透明) */
            ).context("Failed to set the opacity of a layered window.")?;
            ShowWindow(hwnd, SW_SHOW)
                .ok()
                .map_err(|e| anyhow!("Failed to show window - {e}"))?;
            UpdateWindow(hwnd)
                .ok()
                .map_err(|e| anyhow!("Failed to update window - {e}"))?;
        } else {
            return Err(anyhow!(
                "Failed to set 'WS_EX_LAYERED' and 'WS_EX_TRANSPARENT' of window"
            ));
        }
    }
    Ok(())
}

fn load_icon(icon_data: &[u8]) -> Result<Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(icon_data)
            .context("Failed to open icon path")?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Icon::from_rgba(icon_rgba, icon_width, icon_height).context("Failed to crate the logo")
}
