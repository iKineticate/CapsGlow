use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalPosition;

use crate::monitor::MonitorSelector;
use crate::theme::IndicatorTheme;
use crate::window::{WindowPosition, WindowSetting};

pub const WINDOW_LOGICAL_SIZE: f64 = 200.0;

pub static EXE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| std::env::current_exe().expect("Failed to get CapsGlow.exe path"));

pub static EXE_PATH_STRING: LazyLock<String> = LazyLock::new(|| {
    EXE_PATH
        .to_str()
        .map(|s| s.to_string())
        .expect("Failed to EXE 'Path' to 'String'")
});

pub static EXE_NAME: LazyLock<String> = LazyLock::new(|| {
    Path::new(&*EXE_PATH)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_owned())
        .expect("Failed to get EXE name")
});

pub static CONFIG_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| EXE_PATH.with_file_name("CapsGlow.toml"));

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub window_setting: Mutex<WindowSetting>,
    pub indicator_theme: Mutex<IndicatorTheme>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_setting: Mutex::new(WindowSetting::default()),
            indicator_theme: Mutex::new(IndicatorTheme::default()),
        }
    }
}

impl Config {
    pub fn open() -> Result<Self> {
        let default_config = Config::default();

        Config::read().or_else(|e| {
            log::warn!("Failed to read the config file: {e}\nNow creat a new config file");
            let toml_str = toml::to_string_pretty(&default_config)?;
            std::fs::write(&*CONFIG_PATH, toml_str)?;
            Ok(default_config)
        })
    }

    fn read() -> Result<Self> {
        let content = std::fs::read_to_string(&*CONFIG_PATH)?;
        let toml_config: Config = toml::from_str(&content)?;
        Ok(toml_config)
    }

    pub fn save(&self) {
        let toml_str = toml::to_string_pretty(self)
            .expect("Failed to serialize ConfigToml structure as a String of TOML.");
        std::fs::write(&*CONFIG_PATH, toml_str)
            .expect("Failed to write TOML String to CapsGlow.toml");
    }
}

impl Config {
    pub fn is_primary_monitor(&self) -> bool {
        matches!(
            self.window_setting.lock().unwrap().monitor_selector,
            MonitorSelector::PrimaryMonitor
        )
    }

    pub fn is_mouse_monitor(&self) -> bool {
        matches!(
            self.window_setting.lock().unwrap().monitor_selector,
            MonitorSelector::MouseMonitor
        )
    }

    pub fn is_indicator_system_theme(&self) -> bool {
        matches!(
            *self.indicator_theme.lock().unwrap(),
            IndicatorTheme::System
        )
    }

    pub fn is_indicator_indicator_area_theme(&self) -> bool {
        matches!(
            *self.indicator_theme.lock().unwrap(),
            IndicatorTheme::IndicatorArea
        )
    }

    pub fn get_window_position(&self) -> WindowPosition {
        self.window_setting.lock().unwrap().position.clone()
    }

    pub fn get_window_phy_position(
        &self,
        window_phy_width: u32,
        window_phy_height: u32,
    ) -> Result<PhysicalPosition<i32>> {
        self.window_setting
            .lock()
            .unwrap()
            .get_phy_position(window_phy_width, window_phy_height)
    }
}

impl Config {
    pub fn set_primary_monitor(&self) {
        self.window_setting.lock().unwrap().monitor_selector = MonitorSelector::PrimaryMonitor;
    }

    pub fn set_mouse_monitor(&self) {
        self.window_setting.lock().unwrap().monitor_selector = MonitorSelector::MouseMonitor;
    }

    pub fn set_indicator_system_theme(&self) {
        *self.indicator_theme.lock().unwrap() = IndicatorTheme::System;
    }

    pub fn set_indicator_indicator_area_theme(&self) {
        *self.indicator_theme.lock().unwrap() = IndicatorTheme::IndicatorArea;
    }

    pub fn set_window_position(&self, position: WindowPosition) {
        let mut window_setting = self.window_setting.lock().unwrap();
        *window_setting = WindowSetting {
            position,
            monitor_selector: window_setting.monitor_selector.clone(),
        };
    }
}
