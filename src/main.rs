#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod monitor;
mod startup;
mod tray;
mod window;

use crate::{
    monitor::{get_primary_monitor_size, get_scale_factor},
    startup::{is_startup_enabled, set_startup},
    tray::{create_menu, create_tray},
    window::{get_window_center_position, create_window},
};

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use piet_common::{Color, Device, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use tray_icon::menu::MenuEvent;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;

const WINDOW_SIZE: f64 = 200.0;
const TEXT_PADDING: f64 = 20.0;

fn main() -> Result<()> {
    let event_loop = EventLoop::new();

    let scale = get_scale_factor();
    let (pos_x, pos_y) = get_window_center_position(WINDOW_SIZE, scale)?;

    let window = create_window(&event_loop, pos_x, pos_y, WINDOW_SIZE)
        .map_err(|e| anyhow!("Failed to create window - {e}"))?;

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

    let should_startup = is_startup_enabled()?;
    let tray_menu = create_menu(should_startup)?;
    let _tray_icon = create_tray(tray_menu)?;

    let menu_channel = MenuEvent::receiver();

    let event_loop_proxy = event_loop.create_proxy();

    let last_caps_state = Arc::new(Mutex::new(false));

    // Listen globally for CapsLock key activity.
    let last_caps_state_thread = Arc::clone(&last_caps_state);
    std::thread::spawn(move || {
        let last_caps_state = Arc::clone(&last_caps_state_thread);
        loop {
            std::thread::sleep(Duration::from_millis(150));
            // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
            // VK_CAPITAL: 0x14 - CapsLock
            // VK_NUMLOCK: 0x90 - NumLock
            // VK_SCROLL : 0x91 - ScrollLock
            let current_caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
            let mut last_caps_state = last_caps_state.lock().unwrap();
            if current_caps_state != *last_caps_state {
                *last_caps_state = current_caps_state;
                let _ = event_loop_proxy.send_event(());
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));

        if let Ok(menu_event) = menu_channel.try_recv() {
            match menu_event.id().as_ref() {
                "quit" => std::process::exit(0x0100),
                "startup" => {
                    let should_startup =
                        !is_startup_enabled().expect("Failed to get statup status");
                    set_startup(should_startup).expect("Failed to set Launch at Startup")
                }
                _ => (),
            }
        }

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
