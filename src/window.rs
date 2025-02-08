use crate::get_primary_monitor_size;

use anyhow::{anyhow, Context, Result};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::EventLoop,
    platform::windows::WindowBuilderExtWindows,
    platform::windows::WindowExtWindows,
    window::{Window, WindowBuilder},
};
use windows::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR},
    Graphics::Gdi::UpdateWindow,
    UI::WindowsAndMessaging::{
        SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE,
        LAYERED_WINDOW_ATTRIBUTES_FLAGS, SW_SHOW, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    },
};

pub fn get_window_center_position(size: f64, scale: f64) -> Result<(f64, f64)> {
    let (monitor_width, monitor_height) = get_primary_monitor_size()
        .map_err(|e| anyhow!("Failed to get primary monitor size- {e}"))?;
    let window_size_logical = size * scale;
    let pos_x = ((monitor_width - window_size_logical) / 2.0) / scale;
    let pos_y = ((monitor_height - window_size_logical) / 2.0) / scale;
    Ok((pos_x, pos_y))
}

pub fn create_window(event_loop: &EventLoop<()>, x: f64, y: f64, size: f64) -> Result<Window> {
    let window = WindowBuilder::new()
        .with_title("CapsLock")
        .with_skip_taskbar(!cfg!(debug_assertions))
        .with_undecorated_shadow(cfg!(debug_assertions))
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(size, size))
        .with_position(LogicalPosition::new(x, y))
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_focused(false)
        .with_content_protection(true)
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
