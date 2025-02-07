#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use piet_common::{Color, Device, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Window, WindowBuilder},
};
use windows::Win32::{
    Foundation::{GetLastError, SetLastError, HWND, POINT, WIN32_ERROR},
    Graphics::Gdi::{
        GetDC, GetDeviceCaps, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, UpdateWindow,
        LOGPIXELSX, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    },
    UI::{
        Input::KeyboardAndMouse::GetKeyState,
        WindowsAndMessaging::{
            SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE,
            LAYERED_WINDOW_ATTRIBUTES_FLAGS, SW_SHOW, WS_EX_LAYERED, WS_EX_TRANSPARENT,
        },
    },
};

const WINDOW_SIZE: f64 = 200.0;
const TEXT_PADDING: f64 = 20.0;
const ICON_DATA: &[u8] = include_bytes!("logo.ico");

fn main() -> Result<()> {
    let event_loop = EventLoop::new();

    let (monitor_width, monitor_height) = get_primary_monitor_size()
        .map_err(|e| anyhow!("Failed to get primary monitor size- {e}"))?;
    let scale = get_scale_factor();
    let window_size_logical = WINDOW_SIZE * scale;
    let pos_x = ((monitor_width - window_size_logical) / 2.0) / scale;
    let pos_y = ((monitor_height - window_size_logical) / 2.0) / scale;

    let window = create_window(&event_loop, pos_x, pos_y)
        .map_err(|e| anyhow!("Failed to create window - {e}"))?;

    set_mouse_penetrable_layered_window(window.hwnd())
        .map_err(|e| anyhow!("Failed to set mouse penetrable layered window - {e}"))?;

    let (window, _context, mut surface) = {
        let window = std::rc::Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow!("Failed to create a new instance of context - {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow!("Failed to create a surface for drawing to a window - {e}"))?;
        (window, context, surface)
    };

    let mut device =
        Device::new().map_err(|e| anyhow!("Failed to create struct 'Device' - {e}"))?;

    let last_caps_state = Arc::new(Mutex::new(false));

    let event_loop_proxy = event_loop.create_proxy();

    // Listen globally for CapsLock key activity.
    let last_caps_state_thread = Arc::clone(&last_caps_state);
    std::thread::spawn(move || {
        let last_caps_state = Arc::clone(&last_caps_state_thread);
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let current_caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
            let mut last_caps_state = last_caps_state.lock().unwrap();
            if current_caps_state != *last_caps_state {
                *last_caps_state = current_caps_state;
                let _ = event_loop_proxy.send_event(());
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(()) => window.request_redraw(),
            Event::RedrawRequested(_) => {
                let current_caps_state = Arc::clone(&last_caps_state);
                let current_caps_state = *current_caps_state.lock().unwrap();

                let (width, height) = (
                    window.inner_size().width as usize,
                    window.inner_size().height as usize,
                );

                let mut bitmap_target = match device.bitmap_target(width, height, 1.0) {
                    Ok(t) => t,
                    Err(_) => return,
                };
                let mut piet = bitmap_target.render_context();
                piet.clear(None, Color::TRANSPARENT);

                if current_caps_state {
                    let text = piet.text();
                    // Dynamically calculated font size
                    let mut font_size = 10.0;
                    let mut layout;
                    loop {
                        layout = text
                            .new_text_layout("🔒")
                            .font(FontFamily::new_unchecked("Arial"), font_size)
                            .text_color(Color::from_rgba32_u32(0xffffffcc)) // 0xffffff + alpha:00~ff
                            .build()
                            .unwrap();

                        if layout.size().width > (WINDOW_SIZE - TEXT_PADDING) * scale
                            || layout.size().height > (WINDOW_SIZE - TEXT_PADDING) * scale
                        {
                            break;
                        }
                        font_size += 1.0;
                    }

                    let (x, y) = (
                        (width as f64 - layout.size().width) / 2.0,
                        (height as f64 - layout.size().height) / 2.0,
                    );

                    piet.draw_text(&layout, (x, y));
                }

                // Drop the first mutable borrow before the second one
                piet.finish().unwrap();
                drop(piet);

                surface
                    .resize(
                        NonZeroU32::new(width as u32).unwrap(),
                        NonZeroU32::new(height as u32).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();

                if current_caps_state {
                    let buffer_slice = buffer.as_mut();
                    let buffer_slice_u8 = bytemuck::cast_slice_mut(buffer_slice);
                    bitmap_target
                        .copy_raw_pixels(piet_common::ImageFormat::RgbaPremul, buffer_slice_u8) // RgbaSeparate: 颜色分量和透明度是分开存储的，RgbaPremul: 颜色分量已经被透明度乘过。
                        .unwrap();
                } else {
                    buffer.fill(0);
                }

                buffer.present().unwrap();
            }
            _ => (),
        }
    });
}

fn get_primary_monitor_size() -> Result<(f64, f64)> {
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        GetMonitorInfoW(monitor, &mut info).ok()?;

        Ok((
            (info.rcMonitor.right - info.rcMonitor.left) as f64,
            (info.rcMonitor.bottom - info.rcMonitor.top) as f64,
        ))
    }
}

fn get_scale_factor() -> f64 {
    unsafe {
        let hdc = GetDC(None);
        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX) as f64;
        ReleaseDC(None, hdc);
        dpi / 96.0
    }
}

fn create_window(event_loop: &EventLoop<()>, x: f64, y: f64) -> Result<Window> {
    WindowBuilder::new()
        .with_title("CapsLock")
        .with_skip_taskbar(!cfg!(debug_assertions))
        .with_undecorated_shadow(cfg!(debug_assertions))
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
        .with_position(LogicalPosition::new(x, y))
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .build(event_loop)
        .map_err(|e| anyhow::anyhow!("{e}"))
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
                "Failed to set 'WS_EX_LAYERED' and 'WS_EX_TRANSPARENT' of the window"
            ));
        }
    }
    Ok(())
}
