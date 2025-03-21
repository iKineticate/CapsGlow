use crate::language::{Language, Localization};
use crate::startup::get_startup_status;
use crate::ICON_DATA;

use anyhow::{anyhow, Context, Result};
use tray_icon::menu::{IsMenuItem, Submenu};
use tray_icon::{
    menu::{AboutMetadata, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

fn create_menu() -> Result<(Menu, CheckMenuItem, CheckMenuItem)> {
    let should_startup =
        get_startup_status().map_err(|e| anyhow!("Failed to get startup status. - {e}"))?;

    let language = Language::get_system_language();
    let loc = Localization::get(language);

    let tray_menu = Menu::new();
    let menu_quit = MenuItem::with_id("quit", loc.exsit, true, None);
    let menu_separator = PredefinedMenuItem::separator();
    let menu_about = PredefinedMenuItem::about(
        Some(loc.about),
        Some(AboutMetadata {
            name: Some("CapsGlow".to_owned()),
            version: Some("0.1.2".to_owned()),
            authors: Some(vec!["iKineticate".to_owned()]),
            website: Some("https://github.com/iKineticate/CapsGlow".to_owned()),
            ..Default::default()
        }),
    );
    let menu_startup = CheckMenuItem::with_id("startup", loc.startup, true, should_startup, None);

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

    tray_menu
        .append(&menu_theme)
        .context("Failed to apped 'Follow System Theme' to Tray Menu")?;
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

    Ok((
        tray_menu,
        menu_follow_indicator_area_theme,
        menu_follow_system_theme,
    ))
}

pub fn create_tray() -> Result<(TrayIcon, CheckMenuItem, CheckMenuItem)> {
    let (tray_menu, menu_follow_indicator_area_theme, menu_follow_system_theme) =
        create_menu().map_err(|e| anyhow!("Failed to create menu. - {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu_on_left_click(true)
        .with_icon(load_icon(ICON_DATA).map_err(|e| anyhow!("Failed to load icon - {e}"))?)
        .with_tooltip("CapsGlow")
        .with_menu(Box::new(tray_menu))
        .build()
        .map_err(|e| anyhow!("Failed to build tray - {e}"))?;

    Ok((
        tray_icon,
        menu_follow_indicator_area_theme,
        menu_follow_system_theme,
    ))
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
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .context("Failed to crate the logo")
}
