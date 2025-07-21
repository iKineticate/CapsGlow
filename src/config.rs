use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use ini::Ini;

use crate::monitor::{MonitorSelector, WindowPlacement, WindowPosition};
use crate::theme::ThemeDetectionSource;

#[derive(Debug)]
pub struct Config {
    pub config_path: PathBuf,
    pub window_placement: WindowPlacement,
    pub theme_detection_source: ThemeDetectionSource,
}

impl Config {
    pub fn oepn(windows_phy_width: f64, windows_phy_height: f64) -> Result<Self> {
        let config_path = env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(Path::to_path_buf))
            .map(|parent_path| parent_path.join("CapsGlow.ini"))
            .ok_or(anyhow!("Failed to get config path"))?;

        if config_path.is_file() {
            Config::read_ini(config_path, windows_phy_width, windows_phy_height)
        } else {
            Config::create_ini(config_path, windows_phy_width, windows_phy_height)
        }
    }

    pub fn write_setting(&self, key: &str, value: &str) -> Result<()> {
        let ini_path = &self.config_path;
        let mut ini = Ini::load_from_file(ini_path)?;
        ini.set_to(Some("Settings"), key.to_owned(), value.to_owned());
        ini.write_to_file(ini_path)?;
        Ok(())
    }

    fn create_ini(
        ini_path: PathBuf,
        windows_phy_width: f64,
        windows_phy_height: f64,
    ) -> Result<Self> {
        let mut ini = Ini::new();

        ini.with_section(Some("Settings"))
            .set("WindowPosition", "position_center")
            .set("MonitorSelector", "select_mouse_monitor")
            .set("ThemeDetectionSource ", "follow_indicator_area_theme");

        ini.write_to_file(&ini_path)
            .with_context(|| "Failed to create BlueGauge.ini")?;

        Ok(Config {
            config_path: ini_path,
            window_placement: WindowPlacement::new(windows_phy_width, windows_phy_height),
            theme_detection_source: ThemeDetectionSource::IndicatorArea,
        })
    }

    fn read_ini(
        ini_path: PathBuf,
        windows_phy_width: f64,
        windows_phy_height: f64,
    ) -> Result<Self> {
        let ini = Ini::load_from_file(&ini_path).with_context(|| "Failed to load BlueGauge.ini")?;

        let settings = ini
            .section(Some("Settings"))
            .with_context(|| "Failed to get 'Settings' Section")?;

        let window_position = settings
            .get("WindowPosition")
            .and_then(|v| WindowPosition::from_str(v).ok())
            .unwrap_or(WindowPosition::Center);

        let monitor_selector =
            settings
                .get("MonitorSelector")
                .map_or(MonitorSelector::MouseMonitor, |v| match v {
                    "select_mouse_monitor" => MonitorSelector::MouseMonitor,
                    "select_primary_monitor" => MonitorSelector::PrimaryMonitor,
                    _ => MonitorSelector::MouseMonitor,
                });

        let theme_detection_source =
            settings
                .get("ThemeDetectionSource")
                .map_or(ThemeDetectionSource::IndicatorArea, |v| match v {
                    "follow_system_theme" => ThemeDetectionSource::System,
                    "follow_indicator_area_theme" => ThemeDetectionSource::IndicatorArea,
                    _ => ThemeDetectionSource::IndicatorArea,
                });

        Ok(Config {
            config_path: ini_path,
            window_placement: WindowPlacement {
                monitor: monitor_selector,
                position: window_position,
                windows_size: (windows_phy_width, windows_phy_height),
            },
            theme_detection_source,
        })
    }
}
