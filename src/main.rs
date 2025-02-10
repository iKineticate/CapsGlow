#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod font;
mod language;
mod monitor;
mod startup;
mod tray;
mod window;

use crate::{
    font::get_font_bitmap,
    monitor::get_scale_factor,
    startup::{get_startup_status, set_startup},
    tray::create_tray,
    window::create_window,
};

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use piet_common::Device;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};
use tray_icon::menu::MenuEvent;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const WINDOW_LOGICAL_SIZE: f64 = 200.0;
const TEXT_PADDING: f64 = 20.0;
const PERSONALIZE_REGISTRY_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS_USE_LIGHT_THEME_REGISTRY_KEY: &str = "AppsUseLightTheme";
const ICON_DATA: &[u8] = include_bytes!("logo.ico");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

fn main() -> Result<()> {
    let event_loop = EventLoop::new();

    let scale = get_scale_factor();
    let window =
        create_window(&event_loop, scale).map_err(|e| anyhow!("Failed to create window - {e}"))?;
    let mut device = Device::new().map_err(|e| anyhow!("Failed to create device - {e}"))?;
    let (window, _context, mut surface) = {
        let window = std::rc::Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow!("Failed to create a new instance of context - {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow!("Failed to create a surface for drawing to window - {e}"))?;
        (window, context, surface)
    };

    let _tray_icon = create_tray().map_err(|e| anyhow!("Failed to create tray icon. - {e}"))?;
    let menu_channel = MenuEvent::receiver();

    let event_loop_proxy = event_loop.create_proxy();
    let last_caps_state = Arc::new(Mutex::new(false));
    let follow_system_theme = Arc::new(Mutex::new(Some(get_windows_theme())));

    {
        let last_caps_state = Arc::clone(&last_caps_state);
        let follow_system_theme = Arc::clone(&follow_system_theme);
        listen_capslock(last_caps_state, follow_system_theme, event_loop_proxy);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));

        if let Ok(menu_event) = menu_channel.try_recv() {
            match menu_event.id().as_ref() {
                "quit" => *control_flow = ControlFlow::Exit,
                "startup" => {
                    let should_startup =
                        !get_startup_status ().expect("Failed to get startup status");
                    set_startup(should_startup).expect("Failed to set Launch at Startup")
                },
                "theme" => {
                    let mut follow_system_theme = follow_system_theme.lock().unwrap();
                    *follow_system_theme = follow_system_theme.is_none().then(|| get_windows_theme())
                },
                _ => (),
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(()) => window.request_redraw(),
            Event::RedrawRequested(_) => {
                let current_caps_state = *last_caps_state.lock().unwrap();

                let (mut width, mut height) = (window.inner_size().width, window.inner_size().height);
                let window_physical_size = (WINDOW_LOGICAL_SIZE * scale) as u32;
                if width != window_physical_size || height != window_physical_size {
                    window.set_inner_size(LogicalSize::new(WINDOW_LOGICAL_SIZE, WINDOW_LOGICAL_SIZE));
                    (width, height) = (window_physical_size, window_physical_size);
                }

                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .expect("Failed to set the size of the buffer");

                let mut buffer = surface.buffer_mut().unwrap();

                if current_caps_state {
                    let buffer_slice = buffer.as_mut();
                    let buffer_slice_u8 = bytemuck::cast_slice_mut(buffer_slice);
                    let follow_system_theme = Arc::clone(&follow_system_theme);

                    let mut bitmap_target =
                        get_font_bitmap(&mut device, width, height, TEXT_PADDING, scale, follow_system_theme).unwrap();
                    bitmap_target
                        .copy_raw_pixels(piet_common::ImageFormat::RgbaPremul, buffer_slice_u8) // RgbaSeparate: 颜色分量和透明度是分开存储的，RgbaPremul: 颜色分量已经被透明度乘过。
                        .expect("Failed to copy RGBA buffer with premultiplied alpha from font bitmap to surface buffer");

                    window.set_minimized(false);
                } else {
                    buffer.fill(0);
                }

                buffer.present().expect("Failed to presents buffer to the window.");
            }
            _ => (),
        }
    });
}

fn listen_capslock(
    last_caps_state: Arc<Mutex<bool>>,
    follow_system_theme: Arc<Mutex<Option<Theme>>>,
    event_loop_proxy: EventLoopProxy<()>,
) {
    std::thread::spawn(move || {
        let last_caps_state = Arc::clone(&last_caps_state);
        let follow_system_theme = Arc::clone(&follow_system_theme);
        loop {
            std::thread::sleep(Duration::from_millis(150));
            // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
            let current_caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
            let mut last_caps_state = last_caps_state.lock().unwrap();
            if current_caps_state != *last_caps_state {
                if let Ok(mut follow_system_theme) = follow_system_theme.try_lock() {
                    *follow_system_theme = follow_system_theme.and(Some(get_windows_theme()))
                }
                *last_caps_state = current_caps_state;
                event_loop_proxy.send_event(()).unwrap();
            }
        }
    });
}

fn get_windows_theme() -> Theme {
    let personalize_reg_key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(PERSONALIZE_REGISTRY_KEY, KEY_READ | KEY_WRITE)
        .expect("This program requires Windows 10 14393 or above");

    let theme_reg_value: u32 = personalize_reg_key
        .get_value(APPS_USE_LIGHT_THEME_REGISTRY_KEY)
        .expect("This program requires Windows 10 14393 or above");

    match theme_reg_value {
        0 => Theme::Dark,
        _ => Theme::Light,
    }
}
