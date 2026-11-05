pub mod menu;

use crate::{
    icon::load_icon_for_tray,
    tray::menu::{MenuGroup, item::create_menu},
};

use anyhow::{Result, anyhow};
use tray_controls::MenuRegistry;
use tray_icon::{TrayIcon, TrayIconBuilder};
pub fn create_tray(menu_registry: &mut MenuRegistry<MenuGroup>) -> Result<TrayIcon> {
    let icon = load_icon_for_tray()?;

    let tray_menu =
        create_menu(menu_registry).map_err(|e| anyhow!("Failed to create menu. - {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu_on_left_click(true)
        .with_icon(icon)
        .with_tooltip("CapsGlow")
        .with_menu(Box::new(tray_menu))
        .build()
        .map_err(|e| anyhow!("Failed to build tray - {e}"))?;

    Ok(tray_icon)
}
