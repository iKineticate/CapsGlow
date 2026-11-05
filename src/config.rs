use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::icon::IndicatorIcon;
use crate::monitor::MonitorSelector;
use crate::theme::IndicatorTheme;
use crate::window::{WindowPosition, WindowSetting};

pub static CONFIG: LazyLock<RwLock<Config>> =
    LazyLock::new(|| RwLock::new(Config::open().expect("failed to open config")));

pub static EXE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| std::env::current_exe().expect("failed to get CapsGlow.exe path"));

pub static EXE_PATH_STRING: LazyLock<String> = LazyLock::new(|| {
    EXE_PATH
        .to_str()
        .map(|s| s.to_string())
        .expect("failed to EXE 'Path' to 'String'")
});

pub static EXE_DIR: LazyLock<PathBuf> = LazyLock::new(|| EXE_PATH.parent().unwrap().to_path_buf());

pub static EXE_NAME: LazyLock<String> = LazyLock::new(|| {
    Path::new(&*EXE_PATH)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_owned())
        .expect("failed to get EXE name")
});

pub static CONFIG_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| EXE_PATH.with_file_name("CapsGlow.toml"));

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub window_setting: WindowSetting,
    pub indicator_theme: IndicatorTheme,
    pub indicator_icon: IndicatorIcon,
}

impl Config {
    pub fn open() -> Result<Self> {
        let default_config = Config::default();

        Config::read().or_else(|e| {
            log::warn!("failed to read the config file: {e}\nNow creat a new config file");
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
            .expect("failed to serialize ConfigToml structure as a String of TOML.");
        std::fs::write(&*CONFIG_PATH, toml_str)
            .expect("failed to write TOML String to CapsGlow.toml");
    }
}

impl Config {
    pub fn is_primary_monitor(&self) -> bool {
        matches!(
            self.window_setting.monitor_selector,
            MonitorSelector::PrimaryMonitor
        )
    }

    pub fn is_mouse_monitor(&self) -> bool {
        matches!(
            self.window_setting.monitor_selector,
            MonitorSelector::MouseMonitor
        )
    }

    pub fn is_indicator_system_theme(&self) -> bool {
        matches!(self.indicator_theme, IndicatorTheme::System)
    }

    pub fn is_indicator_indicator_area_theme(&self) -> bool {
        matches!(self.indicator_theme, IndicatorTheme::IndicatorArea)
    }

    pub fn get_window_position(&self) -> WindowPosition {
        self.window_setting.position.clone()
    }

    pub fn get_window_phy_size(&self) -> (u32, u32) {
        (
            self.window_setting.phy_width,
            self.window_setting.phy_height,
        )
    }
}

impl Config {
    pub fn set_primary_monitor(&mut self) {
        self.window_setting.monitor_selector = MonitorSelector::PrimaryMonitor;
    }

    pub fn set_mouse_monitor(&mut self) {
        self.window_setting.monitor_selector = MonitorSelector::MouseMonitor;
    }

    pub fn set_indicator_system_theme(&mut self) {
        self.indicator_theme = IndicatorTheme::System;
    }

    pub fn set_indicator_indicator_area_theme(&mut self) {
        self.indicator_theme = IndicatorTheme::IndicatorArea;
    }

    pub fn set_window_position(&mut self, position: WindowPosition) {
        self.window_setting.position = position;
    }

    pub fn set_window_phy_size(&mut self, phy_width: u32, phy_height: u32) {
        self.window_setting.phy_width = phy_width;
        self.window_setting.phy_height = phy_height;
    }
}
