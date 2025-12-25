pub mod menu;

use crate::{
    config::Config,
    icon::load_icon_for_tray,
    tray::menu::{MenuManager, item::create_menu},
};

use anyhow::{Result, anyhow};
use tray_icon::{TrayIcon, TrayIconBuilder};

#[rustfmt::skip]
pub fn create_tray(config: &Config) -> Result<(TrayIcon, MenuManager)> {
    let icon = load_icon_for_tray()?;

    let (tray_menu, tray_check_menus) = create_menu(config).map_err(|e| anyhow!("Failed to create menu. - {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu_on_left_click(true)
        .with_icon(icon)
        .with_tooltip("CapsGlow")
        .with_menu(Box::new(tray_menu))
        .build()
        .map_err(|e| anyhow!("Failed to build tray - {e}"))?;

    Ok((tray_icon, tray_check_menus))
}
