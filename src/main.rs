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
    ffi::OsString,
    num::NonZeroU32,
    process::Command,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    config::{Config, EXE_PATH, WINDOW_LOGICAL_SIZE},
    icon::{CustomIcon, load_icon_for_window, render_font_to_sufface, render_icon_to_buffer},
    monitor::get_scale_factor,
    single_instance::SingleInstance,
    tray::{
        create_tray,
        menu::{MenuManager, about, handler::MenuHandler},
    },
    uiaccess::prepare_uiaccess_token,
};

use anyhow::{Context, Result, anyhow};
use log::error;
use softbuffer::Surface;
use tray_icon::{TrayIcon, menu::MenuEvent};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    platform::windows::{WindowAttributesExtWindows, WindowExtWindows},
    window::{Window, WindowId, WindowLevel},
};

fn main() -> Result<()> {
    let _single_instance = SingleInstance::new()?;

    let _uiaccess_token =
        prepare_uiaccess_token().inspect(|_| println!("Successful acquisition of Uiaccess"));

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::MenuEvent(event))
            .expect("Failed to send MenuEvent");
    }));

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app)?;

    Ok(())
}

struct App {
    config: Arc<Config>,
    exit_threads: Arc<AtomicBool>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
    custom_icon: Option<CustomIcon>,
    menu_manager: Mutex<MenuManager>,
    show_indicator: Arc<AtomicBool>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    tray: Mutex<TrayIcon>,
    window: Option<Rc<Window>>,
    window_phy_height: u32,
    window_phy_width: u32,
}

impl App {
    fn new(event_loop_proxy: EventLoopProxy<UserEvent>) -> Self {
        let config = Config::open().expect("Failed to open config");

        let (tray, menu_manager) = create_tray(&config).expect("Failed to create tray");

        let custom_icon = CustomIcon::find_custom_icon();

        let (window_phy_height, window_phy_width) = custom_icon.as_ref().map_or_else(
            || {
                let scale = get_scale_factor();
                let size = (WINDOW_LOGICAL_SIZE * scale).round() as u32;
                (size, size)
            },
            |i| i.get_size(),
        );

        Self {
            config: Arc::new(config),
            exit_threads: Arc::new(AtomicBool::new(false)),
            event_loop_proxy,
            custom_icon,
            menu_manager: Mutex::new(menu_manager),
            show_indicator: Arc::new(AtomicBool::new(false)),
            surface: None,
            tray: Mutex::new(tray),
            window: None,
            window_phy_height,
            window_phy_width,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window_phy_position = self
            .config
            .window_setting
            .lock()
            .unwrap()
            .get_phy_position(self.window_phy_width, self.window_phy_height)?;

        let window_size = PhysicalSize::new(self.window_phy_width, self.window_phy_height);

        if self.window.is_none() {
            let window = event_loop.create_window(
                Window::default_attributes()
                    .with_title("CapsGlow")
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

            let mut buffer = surface
                .buffer_mut()
                .expect("Failed to get a mutable reference to the buffer");
            buffer.fill(0);
            buffer.present().unwrap();

            self.window = Some(window);
            self.surface = Some(surface);
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit_threads.store(true, Ordering::Relaxed);
    }

    fn listen_capslock(&mut self) {
        let last_show_indicator = Arc::clone(&self.show_indicator);
        let event_loop_proxy = self.event_loop_proxy.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(150));
                // https://learn.microsoft.com/zh-cn/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
                let current_show_indicator = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };
                if current_show_indicator.ne(&last_show_indicator.load(Ordering::Relaxed)) {
                    last_show_indicator.store(current_show_indicator, Ordering::Relaxed);
                    event_loop_proxy
                        .send_event(UserEvent::RedrawRequested)
                        .expect("Failed to send RedrawRequested event");
                }
            }
        });
    }
}

#[derive(Debug)]
enum UserEvent {
    Exit,
    MenuEvent(MenuEvent),
    Restart,
    ShowAboutDialog,
    RedrawRequested,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop)
            .expect("Failed to create window");
        self.listen_capslock();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.exit();
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let window = match self.window.as_ref().filter(|w| w.id() == id) {
                    Some(w) => w,
                    None => return,
                };

                let (window_width, window_height): (u32, u32) = window.inner_size().into();

                let surface = self.surface.as_mut().unwrap();
                let mut buffer = surface.buffer_mut().unwrap();

                if !self.show_indicator.load(Ordering::Relaxed) {
                    buffer.fill(0);
                } else {
                    window.set_skip_taskbar(true);
                    window.set_minimized(false);

                    if let Some(custom_icon) = &self.custom_icon {
                        let theme = self
                            .config
                            .indicator_theme
                            .lock()
                            .unwrap()
                            .get_theme(get_scale_factor(), window_width as f64);

                        let (icon_buffer, icon_size) = custom_icon.get_icon_date_and_size(theme);

                        render_icon_to_buffer(
                            &mut buffer,
                            &icon_buffer,
                            icon_size,
                            window_width,
                            window_height,
                        )
                        .expect("Failed to render icon to surface");
                    } else {
                        let color = self
                            .config
                            .indicator_theme
                            .lock()
                            .unwrap()
                            .get_theme(get_scale_factor(), window_width as f64)
                            .get_font_color();

                        render_font_to_sufface(&mut buffer, color, window_width, window_height)
                            .expect("Failed to render font to surface");
                    }
                }

                buffer.present().expect("Failed to present the buffer");
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Exit => {
                self.exit();
                event_loop.exit();
            }
            UserEvent::MenuEvent(event) => {
                let mut menu_manager = self.menu_manager.lock().unwrap();
                menu_manager.handler(event.id(), |is_normal_menu, check_menu| {
                    let menu_handlers = MenuHandler::new(
                        event.id().clone(),
                        is_normal_menu,
                        check_menu,
                        Arc::clone(&self.config),
                        self.event_loop_proxy.clone(),
                    );
                    if let Err(e) = menu_handlers.run() {
                        error!("Failed to handle menu event: {e}")
                    }
                });
            }
            UserEvent::RedrawRequested => {
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            UserEvent::Restart => {
                let args_os: Vec<OsString> = std::env::args_os().collect();

                if let Err(e) = Command::new(&*EXE_PATH)
                    .args(args_os.iter().skip(1))
                    .spawn()
                {
                    error!("Failed to restart app: {e}");
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
