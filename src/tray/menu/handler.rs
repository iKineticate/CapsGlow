use super::{MenuGroup, item::*};
use crate::{
    UserEvent,
    config::{CONFIG_PATH, Config},
    startup::set_startup,
};

use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use tray_icon::menu::{CheckMenuItem, MenuId};
use winit::event_loop::EventLoopProxy;

pub struct MenuHandler {
    menu_id: MenuId,
    is_normal_menu: bool,
    check_menu: Option<(Option<CheckMenuItem>, Option<MenuGroup>)>,
    config: Arc<Config>,
    proxy: EventLoopProxy<UserEvent>,
}

impl MenuHandler {
    pub fn new(
        menu_id: MenuId,
        is_normal_menu: bool,
        check_menu: Option<(Option<CheckMenuItem>, Option<MenuGroup>)>,
        config: Arc<Config>,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        Self {
            menu_id,
            is_normal_menu,
            check_menu,
            config,
            proxy,
        }
    }

    pub fn run(&self) -> Result<()> {
        let id = &self.menu_id;
        let config = &self.config;
        let proxy = &self.proxy;

        if self.is_normal_menu {
            if id.eq(&*QUIT) {
                proxy
                    .send_event(UserEvent::Exit)
                    .context("Failed to send 'Exit' event")
            } else if id.eq(&*ABOUT) {
                proxy
                    .send_event(UserEvent::ShowAboutDialog)
                    .context("Failed to send 'ShowAboutDialog' event")
            } else if id.eq(&*RESTART) {
                proxy
                    .send_event(UserEvent::Restart)
                    .context("Failed to send 'Restart' event")
            } else if id.eq(&*OPEN_CONFIG) {
                Command::new("notepad.exe")
                    .arg(&*CONFIG_PATH)
                    .spawn()
                    .map(|_| ())
                    .context("Failed to open config file")
            } else {
                Err(anyhow!("No match normal menu: {}", id.0))
            }
        } else if let Some((check_menu, group)) = &self.check_menu {
            if let Some(group) = group {
                match group {
                    // GroupSingle
                    MenuGroup::IndicatorIcon => {
                        if id == &*FOLLOW_INDICATOR_AREA_THEME {
                            config.set_indicator_indicator_area_theme();
                        } else if id == &*FOLLOW_SYSTEM_THEME {
                            config.set_indicator_system_theme();
                        } else {
                            // ...
                        }
                        config.save();
                        Ok(())
                    }
                    // GroupSingle
                    MenuGroup::MonitorSelector => {
                        if id == &*SELECT_MOUSE_MONITOR {
                            config.set_mouse_monitor();
                        } else if id == &*SELECT_PRIMARY_MONITOR {
                            config.set_primary_monitor();
                        } else {
                            // ...
                        }
                        config.save();
                        Ok(())
                    }
                    // GroupSingle
                    MenuGroup::WindowPosition => {
                        if let Some((_, position, _)) = WINDOW_POSITIONS
                            .iter()
                            .find(|(menu_id, _, _)| menu_id == id)
                        {
                            config.set_window_position(position.clone());
                            config.save();
                        }
                        Ok(())
                    }
                }
            } else {
                // 无分组的 CheckMenu
                let Some(check_menu) = check_menu else {
                    return Err(anyhow!(
                        "The clicked CheckMenu no group, but it return GroupSingle(no default): {}",
                        id.0
                    ));
                };

                if id.eq(&*STARTUP) {
                    set_startup(check_menu.is_checked())
                } else {
                    Err(anyhow!("No match single check menu: {}", id.0))
                }
            }
        } else {
            Err(anyhow!("No match any Menu Handler: {}", id.0))
        }
    }
}
