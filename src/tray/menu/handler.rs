use super::{MenuGroup, item::*};
use crate::{
    UserEvent,
    config::{CONFIG, CONFIG_PATH},
    startup::set_startup,
    window::WindowPosition,
};

use std::{process::Command, str::FromStr};

use anyhow::{Context, Result, anyhow};
use tray_controls::MenuItemMeta;
use tray_icon::menu::MenuItemKind;
use winit::event_loop::EventLoopProxy;

pub fn handle_menu_event(
    return_menu_meta: &MenuItemMeta<MenuGroup>,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<()> {
    let return_menu_group = return_menu_meta.group();
    let return_menu_kind = return_menu_meta.kind();
    let return_menu_id = return_menu_kind.id();

    match return_menu_group {
        None => {
            let menu_action = MenuAction::from_str(return_menu_id.as_ref())
                .map_err(|e| anyhow!("no match normal menu [{}] - {e}", return_menu_id.0))?;

            match return_menu_kind {
                MenuItemKind::MenuItem(_m) => match menu_action {
                    MenuAction::Quit => proxy
                        .send_event(UserEvent::Exit)
                        .context("Failed to send 'Exit' event"),
                    MenuAction::About => proxy
                        .send_event(UserEvent::ShowAboutDialog)
                        .context("Failed to send 'Show About Dialog' event"),
                    MenuAction::Restart => proxy
                        .send_event(UserEvent::Restart)
                        .context("Failed to send 'Restart' event"),
                    MenuAction::OpenConfig => Command::new("notepad.exe")
                        .arg(&*CONFIG_PATH)
                        .spawn()
                        .map(|_| ())
                        .context("Failed to open config file"),
                    _ => Err(anyhow!("No match normal menu: {menu_action:?}")),
                },
                MenuItemKind::Check(menu) => {
                    let check_state = menu.is_checked();

                    match menu_action {
                        MenuAction::Startup => set_startup(check_state),
                        _ => Err(anyhow!("No match single check menu: {menu_action:?}")),
                    }
                }
                _ => Err(anyhow!("Unsupported menu kind: {menu_action:?}")),
            }
        }
        Some(group) => {
            // let return_menu = return_menu_kind
            //     .as_check_menuitem()
            //     .ok_or_else(|| anyhow!("Menu is not a check menu: {menu_action:?}"))?;

            // let return_menu_state = return_menu.is_checked();

            let mut config = CONFIG.write().unwrap();

            match group {
                MenuGroup::RadioIndicatorIconTheme => {
                    let select_menu_action = MenuAction::from_str(return_menu_id.as_ref())
                        .map_err(|e| anyhow!("no match radio menu [{}] - {e}", return_menu_id.0))?;

                    match select_menu_action {
                        MenuAction::FollowIndicatorAreaTheme => {
                            config.set_indicator_indicator_area_theme();
                        }
                        MenuAction::FollowSystemTheme => {
                            config.set_indicator_system_theme();
                        }
                        _ => {
                            return Err(anyhow!(
                                "No match set tray icon style menu: {select_menu_action:?}"
                            ));
                        }
                    }

                    config.save();
                    drop(config);

                    proxy
                        .send_event(UserEvent::RedrawRequested)
                        .context("Failed to send 'RedrawRequested' event")
                }
                MenuGroup::RadioMonitorSelector => {
                    let select_menu_action = MenuAction::from_str(return_menu_id.as_ref())
                        .map_err(|e| anyhow!("no match radio menu [{}] - {e}", return_menu_id.0))?;

                    match select_menu_action {
                        MenuAction::SelectMouseMonitor => {
                            config.set_mouse_monitor();
                        }
                        MenuAction::SelectPrimaryMonitor => {
                            config.set_primary_monitor();
                        }
                        _ => {
                            return Err(anyhow!(
                                "No match set monitor selector menu: {select_menu_action:?}"
                            ));
                        }
                    }

                    config.save();
                    drop(config);

                    proxy
                        .send_event(UserEvent::MoveWindow)
                        .context("Failed to send 'Move Window' event")
                }
                MenuGroup::RadioWindowPosition => {
                    let window_position = WindowPosition::from_str(
                        return_menu_id.as_ref().trim_start_matches("position_"),
                    )
                    .map_err(|e| anyhow!("no match radio menu [{}] - {e}", return_menu_id.0))?;

                    config.set_window_position(window_position);
                    config.save();
                    drop(config);

                    proxy
                        .send_event(UserEvent::MoveWindow)
                        .context("Failed to send 'Move Window' event")
                }
            }
        }
    }
}
