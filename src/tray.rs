use crate::ICON_DATA;
use crate::language::{Language, Localization};
use crate::startup::get_startup_status;

use anyhow::{Context, Result, anyhow};
use tray_icon::menu::{IsMenuItem, Submenu};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{AboutMetadata, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
};

fn create_menu() -> Result<(Menu, Vec<CheckMenuItem>)> {
    let should_startup =
        get_startup_status().map_err(|e| anyhow!("Failed to get startup status. - {e}"))?;

    let language = Language::get_system_language();
    let loc = Localization::get(language);

    let tray_menu = Menu::new();

    let menu_separator = PredefinedMenuItem::separator();

    let menu_quit = MenuItem::with_id("quit", loc.exsit, true, None);

    let menu_about = PredefinedMenuItem::about(
        Some(loc.about),
        Some(AboutMetadata {
            name: Some("CapsGlow".to_owned()),
            version: Some("0.2.0".to_owned()),
            authors: Some(vec!["iKineticate".to_owned()]),
            website: Some("https://github.com/iKineticate/CapsGlow".to_owned()),
            ..Default::default()
        }),
    );

    let menu_startup = CheckMenuItem::with_id("startup", loc.startup, true, should_startup, None);

    // 指示器跟随主题
    let menu_follow_indicator_area_theme = CheckMenuItem::with_id(
        "follow_indicator_area_theme",
        loc.follow_indicator_area_theme,
        true,
        true,
        None,
    );

    let menu_follow_system_theme = CheckMenuItem::with_id(
        "follow_system_theme",
        loc.follow_system_theme,
        true,
        false,
        None,
    );

    let menu_theme = Submenu::with_items(
        loc.theme,
        true,
        &[
            &menu_follow_indicator_area_theme as &dyn IsMenuItem,
            &menu_follow_system_theme as &dyn IsMenuItem,
        ],
    )?;

    // 显示位置
    let position = [
        ("position_center", loc.position_center),
        ("position_left", loc.position_left),
        ("position_right", loc.position_right),
        ("position_top", loc.position_top),
        ("position_bottom", loc.position_bottom),
        ("position_top_left", loc.position_top_left),
        ("position_top_right", loc.position_top_right),
        ("position_bottom_left", loc.position_bottom_left),
        ("position_bottom_right", loc.position_bottom_right),
    ];

    // Create owned CheckMenuItems first
    let position_check_items: Vec<CheckMenuItem> = position
        .iter()
        .enumerate()
        .map(|(i, (id, name))| CheckMenuItem::with_id(id, name, true, i == 0, None))
        .collect();

    // Then collect references to them
    let position_check_refs: Vec<&dyn IsMenuItem> = position_check_items
        .iter()
        .map(|item| item as &dyn IsMenuItem)
        .collect();

    let menu_position = Submenu::with_items(loc.position, true, &position_check_refs)?;

    // 屏幕位置选择
    let menu_select_primary_monitor = CheckMenuItem::with_id(
        "select_primary_monitor",
        loc.select_primary_monitor,
        true,
        false,
        None,
    );

    let menu_select_mouse_monitor = CheckMenuItem::with_id(
        "select_mouse_monitor",
        loc.select_mouse_monitor,
        true,
        true,
        None,
    );

    let menu_monitor = Submenu::with_items(
        loc.select_monitor,
        true,
        &[
            &menu_select_primary_monitor as &dyn IsMenuItem,
            &menu_select_mouse_monitor as &dyn IsMenuItem,
        ],
    )?;

    tray_menu
        .append(&menu_position)
        .context("Failed to apped 'Follow System Theme' to Tray Menu")?;
    tray_menu
        .append(&menu_monitor)
        .context("Failed to apped 'Follow System Theme' to Tray Menu")?;
    tray_menu
        .append(&menu_theme)
        .context("Failed to apped 'Follow System Theme' to Tray Menu")?;
    tray_menu
        .append(&menu_separator)
        .context("Failed to apped 'Separator' to Tray Menu")?;
    tray_menu
        .append(&menu_startup)
        .context("Failed to apped 'Launch at Startup' to Tray Menu")?;
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

    let mut tray_check_menus = vec![
        menu_follow_indicator_area_theme,
        menu_follow_system_theme,
        menu_select_primary_monitor,
        menu_select_mouse_monitor,
    ];

    tray_check_menus.extend(position_check_items);

    Ok((tray_menu, tray_check_menus))
}

pub fn create_tray() -> Result<(TrayIcon, Vec<CheckMenuItem>)> {
    let (tray_menu, tray_check_menus) =
        create_menu().map_err(|e| anyhow!("Failed to create menu. - {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu_on_left_click(true)
        .with_icon(load_icon(ICON_DATA).map_err(|e| anyhow!("Failed to load icon - {e}"))?)
        .with_tooltip("CapsGlow")
        .with_menu(Box::new(tray_menu))
        .build()
        .map_err(|e| anyhow!("Failed to build tray - {e}"))?;

    Ok((tray_icon, tray_check_menus))
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
