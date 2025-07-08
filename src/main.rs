#![allow(non_snake_case)]
#![cfg(target_os = "windows")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod font;
mod language;
mod monitor;
mod startup;
mod theme;
mod tray;
mod uiaccess;

use crate::{
    font::render_font_to_sufface,
    monitor::{WindowPlacement, WindowPosition, get_scale_factor},
    startup::{get_startup_status, set_startup},
    theme::{Theme, ThemeDetectionSource, get_indicator_area_theme, get_system_theme},
    tray::create_tray,
    uiaccess::prepare_uiaccess_token,
};

use std::{
    collections::HashMap,
    num::NonZeroU32,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use softbuffer::Surface;
use tray_icon::{
    TrayIcon, TrayIconEvent,
    menu::{CheckMenuItem, MenuEvent},
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::windows::{WindowAttributesExtWindows, WindowExtWindows},
    window::{Icon, Window, WindowId, WindowLevel},
};

const WINDOW_LOGICAL_SIZE: f64 = 200.0;
const TEXT_PADDING: f64 = 20.0;
const ICON_DATA: &[u8] = include_bytes!("logo.ico");

fn main() -> Result<()> {
    let _ = prepare_uiaccess_token().inspect(|_| println!("Successful acquisition of Uiaccess"));

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::TrayIconEvent(event))
            .expect("Failed to send TrayIconEvent");
    }));

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::MenuEvent(event))
            .expect("Failed to send MenuEvent");
    }));

    let mut app = App::default();
    let proxy = event_loop.create_proxy();
    app.add_proxy(Some(proxy));

    event_loop.run_app(&mut app).unwrap();

    Ok(())
}

struct App {
    custom_indicator: Option<HashMap<Vec<u8>, (u64, u64)>>, // (type, (icon_data, width, height))
    event_loop_proxy: Option<EventLoopProxy<UserEvent>>,
    indicator_theme: Arc<Mutex<Option<(ThemeDetectionSource, Theme)>>>,
    window_placement: WindowPlacement,
    scale_factor: f64,
    show_indicator: Arc<AtomicBool>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    tray_icon: Option<TrayIcon>,
    tray_check_menus: Option<Vec<CheckMenuItem>>,
    window: Option<Rc<Window>>,
}

impl Default for App {
    fn default() -> Self {
        let scale_factor = get_scale_factor();
        let indicator_theme = ThemeDetectionSource::IndicatorArea;
        let (tray_icon, tray_check_menus) = create_tray().expect("Failed to create tray icon");

        Self {
            custom_indicator: None,
            event_loop_proxy: None,
            indicator_theme: Arc::new(Mutex::new(Some((
                indicator_theme,
                indicator_theme.get_theme(scale_factor),
            )))),
            window_placement: WindowPlacement::new(WINDOW_LOGICAL_SIZE * scale_factor),
            scale_factor,
            show_indicator: Arc::new(AtomicBool::new(false)),
            surface: None,
            tray_icon: Some(tray_icon),
            tray_check_menus: Some(tray_check_menus),
            window: None,
        }
    }
}

#[derive(Debug)]
enum UserEvent {
    MenuEvent(MenuEvent),
    RedrawRequested,
    TrayIconEvent(TrayIconEvent),
}

impl App {
    fn add_proxy(&mut self, event_loop_proxy: Option<EventLoopProxy<UserEvent>>) -> &mut Self {
        self.event_loop_proxy = event_loop_proxy;
        self
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window_phy_position = self.window_placement.get_phy_position()?;

        if self.window.is_none() {
            let window = event_loop.create_window(
                Window::default_attributes()
                    .with_title("CapsGlow")
                    .with_skip_taskbar(!cfg!(debug_assertions)) // 隐藏任务栏图标
                    .with_undecorated_shadow(cfg!(debug_assertions)) // 隐藏窗口阴影
                    .with_content_protected(!cfg!(debug_assertions)) // 防止窗口被其他应用捕获
                    .with_window_level(WindowLevel::AlwaysOnTop) // 置顶
                    .with_inner_size(LogicalSize::new(WINDOW_LOGICAL_SIZE, WINDOW_LOGICAL_SIZE))
                    .with_min_inner_size(LogicalSize::new(WINDOW_LOGICAL_SIZE, WINDOW_LOGICAL_SIZE))
                    .with_max_inner_size(LogicalSize::new(WINDOW_LOGICAL_SIZE, WINDOW_LOGICAL_SIZE))
                    .with_window_icon(Some(load_icon(ICON_DATA)?))
                    .with_position(window_phy_position)
                    .with_decorations(false) // 隐藏标题栏
                    .with_transparent(true)
                    .with_active(false)
                    .with_resizable(false),
            )?;

            window.set_enable(false);
            window.set_cursor_hittest(false).unwrap();
            window.request_redraw();

            let (window, _context, mut surface) = {
                let window = Rc::new(window);
                let context = softbuffer::Context::new(window.clone())
                    .map_err(|e| anyhow!("Failed to create a new instance of context - {e}"))?;
                let surface = Surface::new(&context, window.clone())
                    .map_err(|e| anyhow!("Failed to create a surface - {e}"))?;
                (window, context, surface)
            };

            let (width, height): (u32, u32) = window.inner_size().into();
            surface
                .resize(
                    NonZeroU32::new(width).with_context(|| "Width must be non-zero")?,
                    NonZeroU32::new(height).with_context(|| "Hight must be non-zero")?,
                )
                .map_err(|e| anyhow!("Failed to set the size of the buffer - {e}"))?;

            let mut buffer = surface.buffer_mut().unwrap();
            buffer.fill(0);
            buffer.present().unwrap();

            self.window = Some(window);
            self.surface = Some(surface);

            self.listen_keys()?;
        }

        Ok(())
    }

    fn listen_keys(&mut self) -> Result<()> {
        let indicator_theme = Arc::clone(&self.indicator_theme);
        let last_show_indicator = Arc::clone(&self.show_indicator);
        let event_loop_proxy = self.event_loop_proxy.clone().unwrap();
        let scale = self.scale_factor;

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(150));
                // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
                let current_show_indicator = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
                if current_show_indicator.ne(&last_show_indicator.load(Ordering::Relaxed)) {
                    if current_show_indicator {
                        let mut indicator_theme = indicator_theme.lock().unwrap();
                        *indicator_theme = indicator_theme.map(|(f, _)| (f, f.get_theme(scale)));
                    }
                    last_show_indicator.store(current_show_indicator, Ordering::Relaxed);
                    event_loop_proxy
                        .send_event(UserEvent::RedrawRequested)
                        .expect("Failed to send RedrawRequested event");
                }
            }
        });

        Ok(())
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop)
            .expect("Failed to create window");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let window = match self.window.as_ref().filter(|w| w.id() == id) {
            Some(w) => w,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let (width, height): (u32, u32) = window.inner_size().into();

                let surface = self.surface.as_mut().unwrap();
                let mut buffer = surface.buffer_mut().unwrap();

                if !self.show_indicator.load(Ordering::Relaxed) {
                    buffer.fill(0);
                    buffer.present().expect("Failed to present the buffer");
                } else {
                    window.set_skip_taskbar(true);
                    window.set_minimized(false);

                    render_font_to_sufface(
                        &mut buffer,
                        width,
                        height,
                        TEXT_PADDING,
                        self.scale_factor,
                        *self.indicator_theme.lock().unwrap(),
                    )
                    .expect("Failed to render font to surface");
                    buffer.present().expect("Failed to present the buffer");
                }
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::RedrawRequested => {
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            UserEvent::TrayIconEvent(_event) => {}
            UserEvent::MenuEvent(event) => {
                let menu_event_id = event.id().as_ref();
                let window = self.window.as_ref().unwrap();
                match menu_event_id {
                    "quit" => event_loop.exit(),
                    "startup" => {
                        let enabled = !get_startup_status().expect("Failed to get startup status");
                        set_startup(enabled).expect("Failed to set Launch at Startup")
                    }
                    "follow_indicator_area_theme" | "follow_system_theme" => {
                        let mut indicator_theme = self.indicator_theme.lock().unwrap();

                        self.tray_check_menus
                            .as_mut()
                            .expect("Tray check menus not initialized")
                            .iter()
                            .filter(|item| item.id().as_ref().ends_with("_theme"))
                            .for_each(|item| {
                                let id = item.id().as_ref();
                                if id == menu_event_id && item.is_checked() {
                                    if id == "follow_indicator_area_theme" {
                                        *indicator_theme = Some((
                                            ThemeDetectionSource::IndicatorArea,
                                            get_indicator_area_theme(self.scale_factor),
                                        ))
                                    } else if id == "follow_system_theme" {
                                        *indicator_theme =
                                            Some((ThemeDetectionSource::System, get_system_theme()))
                                    }
                                } else {
                                    item.set_checked(false)
                                }
                            });
                    }
                    "select_primary_monitor" | "select_mouse_monitor" => {
                        self.tray_check_menus
                            .as_mut()
                            .expect("Tray check menus not initialized")
                            .iter()
                            .filter(|item| item.id().as_ref().ends_with("_monitor"))
                            .for_each(|item| {
                                let id = item.id().as_ref();
                                if id == menu_event_id && item.is_checked() {
                                    if id == "select_primary_monitor" {
                                        self.window_placement.set_primary_monitor();
                                    } else if id == "select_mouse_monitor" {
                                        self.window_placement.set_mouse_monitor();
                                    }
                                } else {
                                    item.set_checked(false)
                                }
                            });

                        let new_position = self
                            .window_placement
                            .get_phy_position()
                            .expect("Failed to get primary monitor position");

                        window.set_outer_position(new_position);
                    }
                    _ if menu_event_id.starts_with("position_") => {
                        let window_position = WindowPosition::from_str(menu_event_id)
                            .expect("Failed to parse window position");

                        self.window_placement.set_window_position(window_position);

                        let new_position = self
                            .window_placement
                            .get_phy_position()
                            .expect("Failed to get primary monitor position");

                        self.tray_check_menus
                            .as_mut()
                            .expect("Tray check menus not initialized")
                            .iter()
                            .filter(|item| item.id().as_ref().starts_with("position_"))
                            .for_each(|item| {
                                if item.id() != menu_event_id {
                                    item.set_checked(false)
                                }
                            });

                        window.set_outer_position(new_position);
                    }
                    _ => (),
                }
            }
        }
    }
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
