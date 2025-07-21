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

#[derive(Debug)]
pub enum MonitorSelector {
    MouseMonitor,
    PrimaryMonitor,
}

#[derive(Debug)]
pub struct WindowPlacement {
    pub monitor: MonitorSelector,
    pub position: WindowPosition,
    pub windows_size: (f64, f64),
}

impl WindowPlacement {
    pub fn new(width: f64, height: f64) -> Self {
        WindowPlacement {
            monitor: MonitorSelector::MouseMonitor,
            position: WindowPosition::Center,
            windows_size: (width, height),
        }
    }

    pub fn get_phy_position(&self) -> Result<PhysicalPosition<f64>> {
        let rect = self.monitor.get_target_monitor_phy_rect()?;
        let (m_left, m_right, m_top, m_bottom) = (
            rect.left as f64,
            rect.right as f64,
            rect.top as f64,
            rect.bottom as f64,
        );
        let (w_width, w_height) = self.windows_size;
        let position = &self.position;

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
            WindowPosition::TopRight => ((m_right - w_width), m_top),
            WindowPosition::BottomLeft => (m_left, (m_top + m_bottom - w_height)),
            WindowPosition::BottomRight => ((m_right - w_width), (m_top + m_bottom - w_height)),
        };
        Ok(PhysicalPosition::new(x, y))
    }

    pub fn set_primary_monitor(&mut self) {
        self.monitor = MonitorSelector::PrimaryMonitor;
    }

    pub fn set_mouse_monitor(&mut self) {
        self.monitor = MonitorSelector::MouseMonitor;
    }
}

#[derive(Debug)]
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

impl WindowPosition {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().trim() {
            "position_center" => Ok(WindowPosition::Center),
            "position_left" => Ok(WindowPosition::Left),
            "position_right" => Ok(WindowPosition::Right),
            "position_top" => Ok(WindowPosition::Top),
            "position_bottom" => Ok(WindowPosition::Bottom),
            "position_top_left" => Ok(WindowPosition::TopLeft),
            "position_top_right" => Ok(WindowPosition::TopRight),
            "position_bottom_left" => Ok(WindowPosition::BottomLeft),
            "position_bottom_right" => Ok(WindowPosition::BottomRight),
            _ => Err(anyhow!("Unknown window position: {s}")),
        }
    }

    pub fn get_str(&self) -> &str {
        match self {
            WindowPosition::Center => "position_center",
            WindowPosition::Left => "position_left",
            WindowPosition::Right => "position_right",
            WindowPosition::Top => "position_top",
            WindowPosition::Bottom => "position_bottom",
            WindowPosition::TopLeft => "position_top_left",
            WindowPosition::TopRight => "position_top_right",
            WindowPosition::BottomLeft => "position_bottom_left",
            WindowPosition::BottomRight => "position_bottom_right",
        }
    }
}

impl MonitorSelector {
    fn get_target_monitor_phy_rect(&self) -> Result<RECT> {
        unsafe {
            let target_cursor = match self {
                MonitorSelector::PrimaryMonitor => Ok(POINT { x: 0, y: 0 }),
                MonitorSelector::MouseMonitor => {
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
}

pub fn get_scale_factor() -> f64 {
    unsafe {
        let hdc = GetDC(None);
        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX) as f64;
        ReleaseDC(None, hdc);
        dpi / 96.0
    }
}
