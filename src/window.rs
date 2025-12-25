use anyhow::Result;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalPosition;

use crate::monitor::MonitorSelector;

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowSetting {
    pub position: WindowPosition,
    pub monitor_selector: MonitorSelector,
}

impl Default for WindowSetting {
    fn default() -> Self {
        Self {
            position: WindowPosition::Center,
            monitor_selector: MonitorSelector::MouseMonitor,
        }
    }
}

impl WindowSetting {
    pub fn get_phy_position(
        &self,
        window_phy_width: u32,
        window_phy_height: u32,
    ) -> Result<PhysicalPosition<i32>> {
        let rect = self.monitor_selector.get_target_monitor_phy_rect()?;
        let (m_left, m_right, m_top, m_bottom) = (rect.left, rect.right, rect.top, rect.bottom);
        let (w_width, w_height) = (window_phy_width as i32, window_phy_height as i32);
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowPosition {
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
