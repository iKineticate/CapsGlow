#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod font;
mod language;
mod monitor;
mod startup;
mod theme;
mod tray;
mod uiaccess;
mod window;

use crate::{
    font::render_font_to_sufface,
    monitor::get_scale_factor,
    startup::{get_startup_status, set_startup},
    theme::{get_indicator_area_theme, get_windows_theme},
    tray::create_tray,
    uiaccess::prepare_uiaccess_token,
    window::create_window,
};

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tao::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy}, platform::windows::WindowExtWindows,
};
use tray_icon::menu::MenuEvent;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;

const WINDOW_LOGICAL_SIZE: f64 = 200.0;
const TEXT_PADDING: f64 = 20.0;
const ICON_DATA: &[u8] = include_bytes!("logo.ico");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeDetectionSource {
    System,
    CenterArea,
}

fn main() -> Result<()> {
    let _ = prepare_uiaccess_token().inspect(|_| println!("Successful acquisition of Uiaccess"));

    let event_loop = EventLoop::new();

    let scale = get_scale_factor();
    let window =
        create_window(&event_loop, scale).map_err(|e| anyhow!("Failed to create window - {e}"))?;
    let (window, _context, mut surface) = {
        let window = std::rc::Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| anyhow!("Failed to create a new instance of context - {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| anyhow!("Failed to create a surface for drawing to window - {e}"))?;
        (window, context, surface)
    };

    let (_tray_icon, menu_follow_indicator_area_theme, menu_follow_system_theme) =
        create_tray().map_err(|e| anyhow!("Failed to create tray icon. - {e}"))?;
    let menu_channel = MenuEvent::receiver();

    let event_loop_proxy = event_loop.create_proxy();
    let last_caps_state = Arc::new(Mutex::new(false));
    let follow_theme = Arc::new(Mutex::new(Some((
        ThemeDetectionSource::CenterArea,
        get_indicator_area_theme(),
    ))));

    listen_capslock(
        Arc::clone(&last_caps_state),
        Arc::clone(&follow_theme),
        event_loop_proxy,
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));

        if let Ok(menu_event) = menu_channel.try_recv() {
            match menu_event.id().as_ref() {
                "quit" => *control_flow = ControlFlow::Exit,
                "startup" => {
                    let enabled = !get_startup_status().expect("Failed to get startup status");
                    set_startup(enabled).expect("Failed to set Launch at Startup")
                }
                "follow_indicator_area_theme" => {
                    let mut follow_theme = follow_theme.lock().unwrap();
                    if menu_follow_indicator_area_theme.is_checked() {
                        menu_follow_system_theme.set_checked(false);
                        *follow_theme =
                            Some((ThemeDetectionSource::CenterArea, get_indicator_area_theme()))
                    } else {
                        *follow_theme = None;
                    }
                }
                "follow_system_theme" => {
                    let mut follow_theme = follow_theme.lock().unwrap();
                    if menu_follow_system_theme.is_checked() {
                        menu_follow_indicator_area_theme.set_checked(false);
                        *follow_theme = Some((ThemeDetectionSource::System, get_windows_theme()))
                    } else {
                        *follow_theme = None
                    }
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
                let window_physical_size = (WINDOW_LOGICAL_SIZE * scale) as u32;
                let (mut width, mut height) = window.inner_size().into();
                if width != window_physical_size || height != window_physical_size {
                    width = window_physical_size;
                    height = window_physical_size;
                    window.set_inner_size(PhysicalSize::new(width, height));
                }

                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .expect("Failed to set the size of the buffer");

                let mut buffer = surface.buffer_mut().unwrap();

                let current_caps_state = *last_caps_state.lock().unwrap();
                if current_caps_state {
                    let follow_theme = *Arc::clone(&follow_theme).lock().unwrap();
                    render_font_to_sufface(
                        &mut buffer,
                        width,
                        height,
                        TEXT_PADDING,
                        scale,
                        follow_theme,
                    )
                    .unwrap();
                    // render_png_to_sufface(&mut buffer, width, height, TEXT_PADDING, scale, follow_theme).unwrap();
                    window.set_minimized(false);
                } else {
                    buffer.fill(0);
                }

                buffer.present().unwrap();
                let _ = window.set_skip_taskbar(true).inspect_err(|e| println!("{e}"));
            }
            _ => (),
        }
    });
}

fn listen_capslock(
    last_caps_state: Arc<Mutex<bool>>,
    follow_theme: Arc<Mutex<Option<(ThemeDetectionSource, Theme)>>>,
    event_loop_proxy: EventLoopProxy<()>,
) {
    std::thread::spawn(move || {
        let last_caps_state = Arc::clone(&last_caps_state);
        let follow_theme = Arc::clone(&follow_theme);
        loop {
            std::thread::sleep(Duration::from_millis(150));
            // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
            let current_caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
            let mut last_caps_state = last_caps_state.lock().unwrap();
            if current_caps_state != *last_caps_state {
                if current_caps_state {
                    if let Ok(mut follow_theme) = follow_theme.try_lock() {
                        *follow_theme = follow_theme.map(|(f, _)| {
                            if f == ThemeDetectionSource::CenterArea {
                                (f, get_indicator_area_theme())
                            } else {
                                (f, get_windows_theme())
                            }
                        });
                    }
                }
                *last_caps_state = current_caps_state;
                event_loop_proxy.send_event(()).unwrap();
            }
        }
    });
}
