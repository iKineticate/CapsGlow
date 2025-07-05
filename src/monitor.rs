use anyhow::{Result, anyhow};
use windows::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::{
        GetDC, GetDeviceCaps, GetMonitorInfoW, LOGPIXELSX, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MonitorFromPoint, ReleaseDC,
    },
    UI::WindowsAndMessaging::GetCursorPos,
};
use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy)]
pub enum MonitorSelector {
    PrimaryMonitor(WindowPosition, f64, f64),
    MouseMonitor(WindowPosition, f64, f64),
}

impl MonitorSelector {
    fn get_window_position(&self) -> WindowPosition {
        match self {
            MonitorSelector::PrimaryMonitor(position, ..) => *position,
            MonitorSelector::MouseMonitor(position, ..) => *position,
        }
    }

    fn get_window_size(&self) -> (f64, f64) {
        match self {
            MonitorSelector::PrimaryMonitor(_, width, height) => (*width, *height),
            MonitorSelector::MouseMonitor(_, width, height) => (*width, *height),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    Custom(PhysicalPosition<f64>),
}

impl MonitorSelector {
    fn get_target_monitor_phy_rect(&self) -> Result<RECT> {
        unsafe {
            let target_cursor = match self {
                MonitorSelector::PrimaryMonitor(_, ..) => Ok(POINT { x: 0, y: 0 }),
                MonitorSelector::MouseMonitor(_, ..) => {
                    let mut point = std::mem::zeroed();
                    GetCursorPos(&mut point)
                        .map(|_| point)
                        .map_err(|e| anyhow!("Failed to get cursor position: {e}"))
                }
            }?;

            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            let monitor = MonitorFromPoint(target_cursor, MONITOR_DEFAULTTONEAREST);

            GetMonitorInfoW(monitor, &mut info).ok()?;

            Ok(info.rcMonitor)
        }
    }

    pub fn get_position(&self) -> Result<PhysicalPosition<f64>> {
        let rect = self.get_target_monitor_phy_rect()?;
        let (m_left, m_right, m_top, m_bottom) = (
            rect.left as f64,
            rect.right as f64,
            rect.top as f64,
            rect.bottom as f64,
        );
        let (w_width, w_height) = self.get_window_size();
        let position = self.get_window_position();

        let (x, y) = match position {
            WindowPosition::Center => (
                ((m_left + m_right - w_width) / 2.0),
                (m_top + m_bottom - w_height) / 2.0,
            ),
            WindowPosition::Left => (m_left, (m_top + m_bottom - w_height) / 2.0),
            WindowPosition::Right => ((m_right - w_width), (m_top + m_bottom - w_height) / 2.0),
            WindowPosition::Top => ((m_left + m_right - w_width) / 2.0, m_top),
            WindowPosition::Bottom => (
                (m_left + m_right - w_width) / 2.0,
                (m_top + m_bottom - w_height),
            ),
            WindowPosition::TopLeft => (m_left, m_top),
            WindowPosition::TopRight => ((m_right - w_width), m_right),
            WindowPosition::BottomLeft => (m_left, (m_top + m_bottom - w_height)),
            WindowPosition::BottomRight => ((m_right - w_width), (m_top + m_bottom - w_height)),
            WindowPosition::Custom(pos) => (pos.x, pos.y),
        };
        Ok(PhysicalPosition::new(x, y))
    }
}

pub fn get_scale_factor() -> f64 {
    unsafe {
        let hdc = GetDC(None);
        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX) as f64;
        ReleaseDC(None, hdc);
        dpi / 96.0
    }
}
