#![allow(non_snake_case)]
#![cfg(target_os = "windows")]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod icon;
mod language;
mod monitor;
mod single_instance;
mod startup;
mod theme;
mod tray;
mod uiaccess;
mod util;
mod window;

use std::{
    cmp::min,
    ffi::OsString,
    num::NonZeroU32,
    process::Command,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use crate::{
    config::{CONFIG, EXE_PATH},
    icon::{
        CustomIcon, FONT_DEFAULT_PATH, IndicatorIcon, check_font_exists, load_icon_for_window,
        render_font_to_surface, render_icon_to_buffer,
    },
    monitor::get_scale_factor,
    single_instance::SingleInstance,
    theme::{IndicatorTheme, get_indicator_area_theme, get_system_theme},
    tray::{
        create_tray,
        menu::{MenuGroup, about, handler::handle_menu_event},
    },
    uiaccess::prepare_uiaccess_token,
};

use anyhow::{Context, Result, anyhow};
use log::error;
use softbuffer::Surface;
use tray_controls::MenuRegistry;
use tray_icon::{TrayIcon, menu::MenuEvent};
use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::GetKeyState};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::windows::{CornerPreference, WindowAttributesExtWindows, WindowExtWindows},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId, WindowLevel},
};

fn main() -> Result<()> {
    let _single_instance = SingleInstance::new()?;

    let _uiaccess_token =
        prepare_uiaccess_token().inspect(|_| log::info!("successful acquisition of Uiaccess"));

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::MenuEvent(event))
            .expect("failed to send MenuEvent");
    }));

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    exit_threads: Arc<AtomicBool>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
    show_indicator: Arc<AtomicBool>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    window: Option<Rc<Window>>,
    close_window_time: Arc<AtomicU64>,
    tray: Mutex<TrayIcon>,
    menu_registry: MenuRegistry<MenuGroup>,
}

impl App {
    fn new(event_loop_proxy: EventLoopProxy<UserEvent>) -> Self {
        let mut menu_registry: MenuRegistry<MenuGroup> = MenuRegistry::new();
        let tray = create_tray(&mut menu_registry).expect("failed to create tray");

        Self {
            close_window_time: Arc::new(AtomicU64::new(0)),
            exit_threads: Arc::new(AtomicBool::new(false)),
            event_loop_proxy,
            menu_registry,
            show_indicator: Arc::new(AtomicBool::new(false)),
            surface: None,
            tray: Mutex::new(tray),
            window: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        if self.window.is_some() {
            return Ok(());
        }

        let window_phy_position = CONFIG.read().unwrap().window_setting.get_phy_position()?;

        let window_size = {
            let (window_phy_width, window_phy_height) =
                CONFIG.read().unwrap().get_window_phy_size();
            PhysicalSize::new(window_phy_width, window_phy_height)
        };

        let window_attributes = Window::default_attributes()
            .with_visible(false)
            .with_title("CapsGlow")
            .with_corner_preference(CornerPreference::DoNotRound)
            .with_skip_taskbar(!cfg!(debug_assertions)) // 隐藏任务栏图标
            .with_undecorated_shadow(cfg!(debug_assertions)) // 隐藏窗口阴影
            .with_content_protected(!cfg!(debug_assertions)) // 防止窗口被其他应用捕获
            .with_window_level(WindowLevel::AlwaysOnTop) // 置顶
            .with_inner_size(window_size)
            .with_min_inner_size(window_size)
            .with_max_inner_size(window_size)
            .with_window_icon(load_icon_for_window().ok())
            .with_position(window_phy_position)
            .with_decorations(false) // 隐藏标题栏
            .with_transparent(true)
            .with_blur(false)
            .with_active(false)
            .with_resizable(false);

        let window = event_loop.create_window(window_attributes)?;

        // 关闭窗口淡入淡出动画
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(win32_handle) = handle.as_raw()
        {
            let hwnd = HWND(win32_handle.hwnd.get() as *mut _);
            let corner_preference = 1i32;
            if let Err(e) = unsafe {
                windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                    hwnd,
                    windows::Win32::Graphics::Dwm::DWMWA_TRANSITIONS_FORCEDISABLED,
                    &corner_preference as *const i32 as *const _,
                    std::mem::size_of::<i32>() as u32,
                )
            } {
                log::error!("failed to set DWMWA_TRANSITIONS_FORCEDISABLED attribute: {e:?}");
            }
        }

        let (window, _context, mut surface) = {
            let window = Rc::new(window);
            let context = softbuffer::Context::new(window.clone())
                .map_err(|e| anyhow!("failed to create a new instance of context - {e}"))?;
            let surface = Surface::new(&context, window.clone())
                .map_err(|e| anyhow!("failed to create a surface - {e}"))?;
            (window, context, surface)
        };

        let (width, height): (u32, u32) = window.inner_size().into();

        CONFIG.write().unwrap().set_window_phy_size(width, height);

        surface
            .resize(
                NonZeroU32::new(width).with_context(|| "Width must be non-zero")?,
                NonZeroU32::new(height).with_context(|| "Hight must be non-zero")?,
            )
            .map_err(|e| anyhow!("failed to set the size of the buffer - {e}"))?;

        window.set_visible(true);
        window.set_enable(false);
        window.set_cursor_hittest(false)?; // 鼠标穿透

        self.window = Some(window);
        self.surface = Some(surface);

        let _ = self.event_loop_proxy.send_event(UserEvent::RedrawRequested);

        Ok(())
    }

    fn exit(&mut self) {
        self.exit_threads.store(true, Ordering::Relaxed);
    }

    fn listen_capslock(&self) {
        let exit_threads = Arc::clone(&self.exit_threads);
        let last_show_indicator = Arc::clone(&self.show_indicator);
        let proxy = self.event_loop_proxy.clone();

        std::thread::spawn(move || {
            while !exit_threads.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(150));
                // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
                let current_show_indicator = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
                if current_show_indicator.ne(&last_show_indicator.load(Ordering::Relaxed)) {
                    last_show_indicator.store(current_show_indicator, Ordering::Relaxed);
                    let _ = proxy.send_event(UserEvent::RedrawRequested);
                }
            }
        });
    }

    fn auto_hide_window(&self) {
        let close_window_time = Arc::clone(&self.close_window_time);
        let exit_threads = Arc::clone(&self.exit_threads);
        let proxy = self.event_loop_proxy.clone();
        let show_indicator = Arc::clone(&self.show_indicator);

        std::thread::spawn(move || {
            while !exit_threads.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_mins(1));

                if close_window_time.fetch_add(1, Ordering::Relaxed) >= 1
                    && !show_indicator.load(Ordering::Relaxed)
                {
                    close_window_time.store(0, Ordering::Relaxed);
                    let _ = proxy.send_event(UserEvent::HideWindow);
                }
            }
        });
    }
}

#[derive(Debug)]
enum UserEvent {
    HideWindow,
    Exit,
    MenuEvent(MenuEvent),
    MoveWindow,
    Restart,
    ShowAboutDialog,
    RedrawRequested,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 如果窗口已存在，说明是从睡眠/挂起中恢复
        if self.window.is_some() {
            log::info!("App resumed from suspension, recreating surface...");

            // 关键：重建 surface 以修复休眠导致的图形上下文丢失
            if let Some(window) = &self.window {
                match softbuffer::Context::new(window.clone()) {
                    Ok(context) => match Surface::new(&context, window.clone()) {
                        Ok(mut new_surface) => {
                            let (width, height): (u32, u32) = window.inner_size().into();
                            if let (Some(w), Some(h)) =
                                (NonZeroU32::new(width), NonZeroU32::new(height))
                                && new_surface.resize(w, h).is_ok()
                            {
                                self.surface = Some(new_surface);
                                log::info!("Surface recreated successfully after resume");
                            }
                        }
                        Err(e) => log::error!("failed to recreate surface on resume: {e}"),
                    },
                    Err(e) => log::error!("failed to recreate context on resume: {e}"),
                }
            }

            // 强制触发一次重绘
            let _ = self.event_loop_proxy.send_event(UserEvent::RedrawRequested);
        } else {
            // 首次启动
            self.create_window(event_loop)
                .expect("failed to create window");
            self.listen_capslock();
            self.auto_hide_window();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.exit();
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // WARN: 发送 windows.request_redraw() 请求重绘，如果托盘菜单正在打开中，Windows 消息循环（Message Loop）被阻塞，会导致重绘失败
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::HideWindow => {
                if let Some(window) = self.window.as_ref() {
                    window.set_visible(false);
                    log::info!("Window set invisible");
                }
            }
            UserEvent::Exit => {
                self.exit();
                event_loop.exit();
            }
            UserEvent::MenuEvent(event) => {
                if let Err(e) = self
                    .menu_registry
                    .handle_event(event.id())
                    .map_err(|e| anyhow!("{e}"))
                    .and_then(|menu_meta| handle_menu_event(menu_meta, &self.event_loop_proxy))
                {
                    error!("failed to handle menu event: {e}");
                }
            }
            UserEvent::MoveWindow => {
                if let Some(window) = self.window.as_ref() {
                    let window_phy_position = CONFIG
                        .read()
                        .unwrap()
                        .window_setting
                        .get_phy_position()
                        .expect("failed to get window physical position");

                    window.set_outer_position(window_phy_position);
                }
            }
            UserEvent::RedrawRequested => {
                if let Some(window) = self.window.as_ref() {
                    window.set_visible(true);

                    let (window_width, window_height): (u32, u32) = window.inner_size().into();

                    let surface = self.surface.as_mut().unwrap();
                    let mut buffer = surface.buffer_mut().unwrap();

                    buffer.fill(0);

                    if self.show_indicator.load(Ordering::Relaxed) {
                        window.set_skip_taskbar(true);
                        window.set_minimized(false);

                        let config = CONFIG.read().unwrap();

                        let icon_theme = match config.indicator_theme {
                            IndicatorTheme::System => get_system_theme(),
                            IndicatorTheme::IndicatorArea => get_indicator_area_theme(
                                get_scale_factor(),
                                min(window_width, window_height) as f64,
                            ),
                        };

                        match config.indicator_icon {
                            IndicatorIcon::Font {
                                ref glyph,
                                ref font_path,
                            } => {
                                let font_color = icon_theme.get_font_color();

                                let font_path = check_font_exists(font_path)
                                    .unwrap_or_else(|| FONT_DEFAULT_PATH.to_owned());

                                if let Err(e) = render_font_to_surface(
                                    &mut buffer,
                                    *glyph,
                                    font_color,
                                    font_path,
                                    window_width,
                                    window_height,
                                ) {
                                    log::error!("failed to render font to surface: {e}")
                                };
                            }
                            IndicatorIcon::Custom(ref custom_icon) => {
                                let icon_path = match custom_icon {
                                    CustomIcon::Normal(image_path) => image_path,
                                    CustomIcon::Theme { light, dark } => match icon_theme {
                                        theme::Theme::Light => light,
                                        theme::Theme::Dark => dark,
                                    },
                                };

                                if let Ok(image) = image::open(icon_path) {
                                    if let Err(e) = render_icon_to_buffer(
                                        &mut buffer,
                                        image,
                                        window_width,
                                        window_height,
                                    ) {
                                        log::error!("failed to render icon to surface: {e}")
                                    };
                                } else {
                                    log::error!("failed to open icon file");
                                }
                            }
                        }
                    }

                    buffer.present().expect("failed to present the buffer");
                } else {
                    self.create_window(event_loop)
                        .expect("failed to create window");
                }
            }
            UserEvent::Restart => {
                let args_os: Vec<OsString> = std::env::args_os().collect();

                if let Err(e) = Command::new(&*EXE_PATH)
                    .args(args_os.iter().skip(1))
                    .spawn()
                {
                    error!("failed to restart app: {e}");
                }

                let _ = self.event_loop_proxy.send_event(UserEvent::Exit);
            }
            UserEvent::ShowAboutDialog => {
                let hwnd = self.tray.lock().unwrap().window_handle();
                about::show_about_dialog(hwnd as isize);
            }
        }
    }
}
