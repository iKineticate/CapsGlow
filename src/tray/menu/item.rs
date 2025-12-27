use super::{MenuGroup, MenuKind, MenuManager};
use crate::language::LOC;
use crate::startup::get_startup_status;
use crate::{config::Config, window::WindowPosition};

use std::sync::LazyLock;

use anyhow::{Context, Result};
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

pub static QUIT: LazyLock<MenuId> = LazyLock::new(|| MenuId::new("quit")); // Normal
pub static ABOUT: LazyLock<MenuId> = LazyLock::new(|| MenuId::new("about")); // Normal
pub static RESTART: LazyLock<MenuId> = LazyLock::new(|| MenuId::new("restart")); // Normal
pub static STARTUP: LazyLock<MenuId> = LazyLock::new(|| MenuId::new("startup")); // CheckSingle
// Normal
pub static OPEN_CONFIG: LazyLock<MenuId> = LazyLock::new(|| MenuId::new("open_config"));
// Indicator Theme: GroupSingle
pub static FOLLOW_INDICATOR_AREA_THEME: LazyLock<MenuId> =
    LazyLock::new(|| MenuId::new("follow_indicator_area_theme"));
pub static FOLLOW_SYSTEM_THEME: LazyLock<MenuId> =
    LazyLock::new(|| MenuId::new("follow_system_theme"));
// Monitor GroupSingle: GroupSingle
pub static SELECT_MOUSE_MONITOR: LazyLock<MenuId> =
    LazyLock::new(|| MenuId::new("select_mouse_monitor"));
pub static SELECT_PRIMARY_MONITOR: LazyLock<MenuId> =
    LazyLock::new(|| MenuId::new("select_primary_monitor"));
// Window Position: GroupSingle
pub static WINDOW_POSITIONS: LazyLock<[(MenuId, WindowPosition, &str); 9]> = LazyLock::new(|| {
    [
        (
            MenuId::new("position_center"),
            WindowPosition::Center,
            LOC.position_center,
        ),
        (
            MenuId::new("position_left"),
            WindowPosition::Left,
            LOC.position_left,
        ),
        (
            MenuId::new("position_right"),
            WindowPosition::Right,
            LOC.position_right,
        ),
        (
            MenuId::new("position_top"),
            WindowPosition::Top,
            LOC.position_top,
        ),
        (
            MenuId::new("position_bottom"),
            WindowPosition::Bottom,
            LOC.position_bottom,
        ),
        (
            MenuId::new("position_top_left"),
            WindowPosition::TopLeft,
            LOC.position_top_left,
        ),
        (
            MenuId::new("position_top_right"),
            WindowPosition::TopRight,
            LOC.position_top_right,
        ),
        (
            MenuId::new("position_bottom_left"),
            WindowPosition::BottomLeft,
            LOC.position_bottom_left,
        ),
        (
            MenuId::new("position_bottom_right"),
            WindowPosition::BottomRight,
            LOC.position_bottom_right,
        ),
    ]
});

struct CreateMenuItem(MenuManager);

impl CreateMenuItem {
    fn new() -> Self {
        Self(MenuManager::new())
    }

    fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::separator()
    }

    fn quit(&mut self, text: &str) -> MenuItem {
        self.0.insert(QUIT.clone(), MenuKind::Normal, None);
        MenuItem::with_id(QUIT.clone(), text, true, None)
    }

    fn about(&mut self, text: &str) -> MenuItem {
        self.0.insert(ABOUT.clone(), MenuKind::Normal, None);
        MenuItem::with_id(ABOUT.clone(), text, true, None)
    }

    fn restart(&mut self, text: &str) -> MenuItem {
        self.0.insert(RESTART.clone(), MenuKind::Normal, None);
        MenuItem::with_id(RESTART.clone(), text, true, None)
    }

    fn open_config(&mut self, text: &str) -> MenuItem {
        self.0.insert(OPEN_CONFIG.clone(), MenuKind::Normal, None);
        MenuItem::with_id(OPEN_CONFIG.clone(), text, true, None)
    }

    fn startup(&mut self, text: &str) -> Result<CheckMenuItem> {
        let should_startup = get_startup_status()?;
        let menu_id = STARTUP.clone();
        let menu = CheckMenuItem::with_id(menu_id.clone(), text, true, should_startup, None);
        self.0
            .insert(STARTUP.clone(), MenuKind::CheckSingle, Some(menu.clone()));
        Ok(menu)
    }

    fn indicator_theme(&mut self, config: &Config) -> Result<Submenu> {
        let menu_follow_indicator_area_theme = CheckMenuItem::with_id(
            FOLLOW_INDICATOR_AREA_THEME.clone(),
            LOC.follow_indicator_area_theme,
            true,
            config.is_indicator_indicator_area_theme(),
            None,
        );

        let menu_follow_system_theme = CheckMenuItem::with_id(
            FOLLOW_SYSTEM_THEME.clone(),
            LOC.follow_system_theme,
            true,
            config.is_indicator_system_theme(),
            None,
        );

        self.0.insert(
            FOLLOW_INDICATOR_AREA_THEME.clone(),
            MenuKind::GroupSingle(
                MenuGroup::IndicatorIcon,
                Some(FOLLOW_INDICATOR_AREA_THEME.clone()),
            ),
            Some(menu_follow_indicator_area_theme.clone()),
        );
        self.0.insert(
            FOLLOW_SYSTEM_THEME.clone(),
            MenuKind::GroupSingle(
                MenuGroup::IndicatorIcon,
                Some(FOLLOW_INDICATOR_AREA_THEME.clone()),
            ),
            Some(menu_follow_system_theme.clone()),
        );

        Submenu::with_items(
            LOC.theme,
            true,
            &[
                &menu_follow_indicator_area_theme as &dyn IsMenuItem,
                &menu_follow_system_theme as &dyn IsMenuItem,
            ],
        )
        .context("Failed to apped 'Indicator Theme' to Tray Menu")
    }

    fn window_postion(&mut self, config: &Config) -> Result<Submenu> {
        let position_check_items = WINDOW_POSITIONS
            .iter()
            .map(|(menu_id, position, text)| {
                let menu = CheckMenuItem::with_id(
                    menu_id.clone(),
                    text,
                    true,
                    config.get_window_position() == *position,
                    None,
                );
                self.0.insert(
                    menu_id.clone(),
                    MenuKind::GroupSingle(
                        MenuGroup::WindowPosition,
                        Some(MenuId::new("position_center")),
                    ),
                    Some(menu.clone()),
                );
                menu
            })
            .collect::<Vec<CheckMenuItem>>();

        let position_check_refs: Vec<&dyn IsMenuItem> = position_check_items
            .iter()
            .map(|item| item as &dyn IsMenuItem)
            .collect();

        Submenu::with_items(LOC.position, true, &position_check_refs)
            .context("Failed to apped 'Window Postion' to Tray Menu")
    }

    fn select_monitor(&mut self, config: &Config) -> Result<Submenu> {
        let menu_select_primary_monitor = CheckMenuItem::with_id(
            SELECT_PRIMARY_MONITOR.clone(),
            LOC.select_primary_monitor,
            true,
            config.is_primary_monitor(),
            None,
        );

        let menu_select_mouse_monitor = CheckMenuItem::with_id(
            SELECT_MOUSE_MONITOR.clone(),
            LOC.select_mouse_monitor,
            true,
            config.is_mouse_monitor(),
            None,
        );

        self.0.insert(
            SELECT_PRIMARY_MONITOR.clone(),
            MenuKind::GroupSingle(
                MenuGroup::MonitorSelector,
                Some(SELECT_MOUSE_MONITOR.clone()),
            ),
            Some(menu_select_primary_monitor.clone()),
        );
        self.0.insert(
            SELECT_MOUSE_MONITOR.clone(),
            MenuKind::GroupSingle(
                MenuGroup::MonitorSelector,
                Some(SELECT_MOUSE_MONITOR.clone()),
            ),
            Some(menu_select_mouse_monitor.clone()),
        );

        Submenu::with_items(
            LOC.select_monitor,
            true,
            &[
                &menu_select_primary_monitor as &dyn IsMenuItem,
                &menu_select_mouse_monitor as &dyn IsMenuItem,
            ],
        )
        .context("Failed to apped 'Select Monitor' to Tray Menu")
    }
}

pub fn create_menu(config: &Config) -> Result<(Menu, MenuManager)> {
    let menu_separator = CreateMenuItem::separator();

    let mut create_menu_item = CreateMenuItem::new();

    let menu_about = create_menu_item.about(LOC.about);

    let menu_quit = create_menu_item.quit(LOC.quit);

    let menu_restart = create_menu_item.restart(LOC.restart);

    let menu_startup = create_menu_item.startup(LOC.startup)?;

    let menu_open_config = create_menu_item.open_config(LOC.open_config);

    let menu_indicator_theme = create_menu_item.indicator_theme(config)?;

    let menu_window_position = create_menu_item.window_postion(config)?;

    let menu_select_monitor = create_menu_item.select_monitor(config)?;

    let tray_menu = Menu::new();

    tray_menu
        .append(&menu_select_monitor)
        .context("Failed to apped 'Select Monitor up' to Tray Menu")?;
    tray_menu
        .append(&menu_window_position)
        .context("Failed to apped 'Window Postion' to Tray Menu")?;
    tray_menu
        .append(&menu_indicator_theme)
        .context("Failed to apped 'Indicator Theme' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_open_config)
        .context("Failed to apped 'Open Config' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_startup)
        .context("Failed to apped 'Satr up' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_restart)
        .context("Failed to apped 'Restart' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_about)
        .context("Failed to apped 'About' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_quit)
        .context("Failed to apped 'Quit' to Tray Menu")?;

    Ok((tray_menu, create_menu_item.0))
}
