use super::MenuGroup;
use crate::language::LOC;
use crate::startup::get_startup_status;
use crate::{config::Config, window::WindowPosition};

use anyhow::{Context, Result};
use strum::{AsRefStr, EnumString, IntoEnumIterator};
use tray_controls::MenuRegistry;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};

#[derive(Debug, Clone, Eq, Hash, PartialEq, AsRefStr, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum MenuAction {
    // Normal
    Quit,
    About,
    Restart,
    OpenConfig,
    // CheckSingle
    Startup,
    // Radio
    FollowIndicatorAreaTheme,
    FollowSystemTheme,
    // Radio
    SelectMouseMonitor,
    SelectPrimaryMonitor,
    // 注意 WindowPosition 在 window.rs 中已经实现了 AsRefStr 和 EnumString
    // WindowPosition,
}

impl MenuAction {
    pub fn id(&self) -> MenuId {
        MenuId::new(self.as_ref())
    }
}

struct CreateMenuItem(MenuRegistry<MenuGroup>);

impl CreateMenuItem {
    fn new() -> Self {
        Self(MenuRegistry::<MenuGroup>::new())
    }

    fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::separator()
    }

    fn quit(&mut self, text: &str) -> MenuItem {
        let menu_item = MenuItem::with_id(MenuAction::Quit.id(), text, true, None);
        self.0.register_normal(menu_item.kind());
        menu_item
    }

    fn about(&mut self, text: &str) -> MenuItem {
        let menu_item = MenuItem::with_id(MenuAction::About.id(), text, true, None);
        self.0.register_normal(menu_item.kind());
        menu_item
    }

    fn restart(&mut self, text: &str) -> MenuItem {
        let menu_item = MenuItem::with_id(MenuAction::Restart.id(), text, true, None);
        self.0.register_normal(menu_item.kind());
        menu_item
    }

    fn open_config(&mut self, text: &str) -> MenuItem {
        let menu_item = MenuItem::with_id(MenuAction::OpenConfig.id(), text, true, None);
        self.0.register_normal(menu_item.kind());
        menu_item
    }

    fn startup(&mut self, text: &str) -> Result<CheckMenuItem> {
        let should_startup = get_startup_status()?;
        let check_menu_item =
            CheckMenuItem::with_id(MenuAction::Startup.id(), text, true, should_startup, None);
        self.0.register_normal(check_menu_item.kind());
        Ok(check_menu_item)
    }

    fn indicator_theme(&mut self, config: &Config) -> Result<Submenu> {
        let menu_radio_items = [
            (
                MenuAction::FollowIndicatorAreaTheme,
                config.is_indicator_indicator_area_theme(),
            ),
            (
                MenuAction::FollowSystemTheme,
                config.is_indicator_system_theme(),
            ),
        ]
        .into_iter()
        .map(|(menu_action, checked)| {
            let menu_id = menu_action.id();
            let text = LOC.get(menu_action.as_ref()).unwrap_or("unknown");
            let menu = CheckMenuItem::with_id(menu_id, text, true, checked, None);

            self.0.register_radio(
                menu.kind(),
                MenuGroup::RadioIndicatorIconTheme,
                Some(MenuAction::FollowIndicatorAreaTheme.id()),
            );

            menu
        })
        .collect::<Vec<CheckMenuItem>>();

        let menu_tray_icon_style: Vec<&dyn IsMenuItem> = menu_radio_items
            .iter()
            .map(|item| item as &dyn IsMenuItem)
            .collect();

        Submenu::with_items(LOC.theme, true, &menu_tray_icon_style)
            .context("Failed to apped 'Indicator Theme' to Tray Menu")
    }

    fn window_postion(&mut self, config: &Config) -> Result<Submenu> {
        let menu_radio_items = WindowPosition::iter()
            .map(|position| {
                let menu_id = MenuId::new(position.as_ref());

                let text = LOC.get(position.as_ref()).unwrap_or("unknown");

                let menu = CheckMenuItem::with_id(
                    menu_id,
                    text,
                    true,
                    config.get_window_position() == position,
                    None,
                );

                self.0.register_radio(
                    menu.kind(),
                    MenuGroup::RadioWindowPosition,
                    Some(MenuId(WindowPosition::Center.as_ref().to_string())),
                );

                menu
            })
            .collect::<Vec<CheckMenuItem>>();

        let position_check_refs: Vec<&dyn IsMenuItem> = menu_radio_items
            .iter()
            .map(|item| item as &dyn IsMenuItem)
            .collect();

        Submenu::with_items(LOC.position, true, &position_check_refs)
            .context("Failed to apped 'Window Postion' to Tray Menu")
    }

    fn select_monitor(&mut self, config: &Config) -> Result<Submenu> {
        let menu_radio_items = [
            (
                MenuAction::SelectPrimaryMonitor,
                config.is_primary_monitor(),
            ),
            (MenuAction::SelectMouseMonitor, config.is_mouse_monitor()),
        ]
        .into_iter()
        .map(|(menu_action, checked)| {
            let menu_id = menu_action.id();
            let text = LOC.get(menu_action.as_ref()).unwrap_or("unknown");

            let menu = CheckMenuItem::with_id(menu_id, text, true, checked, None);

            self.0.register_radio(
                menu.kind(),
                MenuGroup::RadioMonitorSelector,
                Some(MenuAction::SelectPrimaryMonitor.id()),
            );

            menu
        })
        .collect::<Vec<CheckMenuItem>>();

        Submenu::with_items(
            LOC.select_monitor,
            true,
            &[
                &menu_radio_items[0] as &dyn IsMenuItem,
                &menu_radio_items[1] as &dyn IsMenuItem,
            ],
        )
        .context("Failed to apped 'Select Monitor' to Tray Menu")
    }
}

pub fn create_menu(config: &Config, menu_manager: &mut MenuRegistry<MenuGroup>) -> Result<Menu> {
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

    *menu_manager = create_menu_item.0;

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

    Ok(tray_menu)
}
