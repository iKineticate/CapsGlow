use anyhow::Result;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalPosition;

use crate::monitor::{MonitorSelector, get_scale_factor};

pub const WINDOW_LOGICAL_SIZE: f64 = 200.0;

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSetting {
    pub phy_width: u32,
    pub phy_height: u32,
    pub position: WindowPosition,
    pub monitor_selector: MonitorSelector,
}

impl Default for WindowSetting {
    fn default() -> Self {
        let (window_phy_width, window_phy_height) = {
            let scale = get_scale_factor();
            let size = (WINDOW_LOGICAL_SIZE * scale).round() as u32;
            (size, size)
        };

        Self {
            phy_width: window_phy_width,
            phy_height: window_phy_height,
            position: WindowPosition::Center,
            monitor_selector: MonitorSelector::MouseMonitor,
        }
    }
}

impl WindowSetting {
    pub fn get_phy_position(&self) -> Result<PhysicalPosition<i32>> {
        let rect = self.monitor_selector.get_target_monitor_phy_rect()?;
        let (m_left, m_right, m_top, m_bottom) = (rect.left, rect.right, rect.top, rect.bottom);
        let (w_width, w_height) = (self.phy_width as i32, self.phy_height as i32);
        let position = &self.position;

        let (x, y) = match position {
            WindowPosition::Center => (
                ((m_left + m_right - w_width) / 2),
                (m_top + m_bottom - w_height) / 2,
            ),
            WindowPosition::Left => (m_left, (m_top + m_bottom - w_height) / 2),
            WindowPosition::Right => ((m_right - w_width), (m_top + m_bottom - w_height) / 2),
            WindowPosition::Top => ((m_left + m_right - w_width) / 2, m_top),
            WindowPosition::Bottom => (
                (m_left + m_right - w_width) / 2,
                (m_top + m_bottom - w_height),
            ),
            WindowPosition::TopLeft => (m_left, m_top),
            WindowPosition::TopRight => ((m_right - w_width), m_top),
            WindowPosition::BottomLeft => (m_left, (m_top + m_bottom - w_height)),
            WindowPosition::BottomRight => ((m_right - w_width), (m_top + m_bottom - w_height)),
        };
        Ok(PhysicalPosition::new(x, y))
    }
}

use strum::{AsRefStr, EnumIter, EnumString};

#[derive(
    Default,
    Debug,
    Clone,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    AsRefStr,
    EnumIter,
    EnumString,
)]
#[strum(prefix = "position_", serialize_all = "snake_case")]
pub enum WindowPosition {
    #[default]
    Center,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
