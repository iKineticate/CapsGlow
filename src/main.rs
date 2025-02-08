#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod font;
mod monitor;
mod startup;
mod tray;
mod window;

use crate::{
    font::get_font_bitmap,
    monitor::{get_primary_monitor_size, get_scale_factor},
    startup::{is_startup_enabled, set_startup},
    tray::{create_menu, create_tray},
    window::{create_window, get_window_center_position},
};

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use piet_common::Device;
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
    let (pos_x, pos_y) = get_window_center_position(WINDOW_SIZE, scale)
        .map_err(|e| anyhow!("Failed to get window center position - {e}"))?;

    let window = create_window(&event_loop, pos_x, pos_y, WINDOW_SIZE)
        .map_err(|e| anyhow!("Failed to create window - {e}"))?;

    let (window, _context, mut surface) = {
        let window = std::rc::Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow!("Failed to create a new instance of context - {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow!("Failed to create a surface for drawing to window - {e}"))?;
        (window, context, surface)
    };

    let mut device =
        Device::new().map_err(|e| anyhow!("Failed to create struct 'Device' - {e}"))?;

    let should_startup =
        is_startup_enabled().map_err(|e| anyhow!("Failed to get startup status. - {e}"))?;
    let tray_menu =
        create_menu(should_startup).map_err(|e| anyhow!("Failed to create menu. - {e}"))?;
    let _tray_icon =
        create_tray(tray_menu).map_err(|e| anyhow!("Failed to create tray icon. - {e}"))?;

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
            let current_caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
            let mut last_caps_state = last_caps_state.lock().unwrap();
            if current_caps_state != *last_caps_state {
                *last_caps_state = current_caps_state;
                event_loop_proxy.send_event(()).unwrap();
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));

        if let Ok(menu_event) = menu_channel.try_recv() {
            match menu_event.id().as_ref() {
                "quit" => *control_flow = ControlFlow::Exit,
                "startup" => {
                    let should_startup =
                        !is_startup_enabled().expect("Failed to get startup status");
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

                let (width, height) = (window.inner_size().width, window.inner_size().height);

                surface
                    .resize(
                        NonZeroU32::new(width as u32).unwrap(),
                        NonZeroU32::new(height as u32).unwrap(),
                    )
                    .expect("Failed to set the size of the buffer");

                let mut buffer = surface.buffer_mut().unwrap();

                if current_caps_state {
                    let buffer_slice = buffer.as_mut();
                    let buffer_slice_u8 = bytemuck::cast_slice_mut(buffer_slice);

                    let mut bitmap_target =
                        get_font_bitmap(&mut device, width, height, TEXT_PADDING, scale).unwrap();
                    bitmap_target
                        .copy_raw_pixels(piet_common::ImageFormat::RgbaPremul, buffer_slice_u8) // RgbaSeparate: 颜色分量和透明度是分开存储的，RgbaPremul: 颜色分量已经被透明度乘过。
                        .expect("Failed to copy RGBA buffer with premultiplied alpha from font bitmap to surface buffer");
                } else {
                    buffer.fill(0);
                }

                buffer.present().expect("Failed to presents buffer to the window.");
            }
            _ => (),
        }
    });
}
