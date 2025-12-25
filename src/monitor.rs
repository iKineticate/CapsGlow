use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use windows::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::{
        GetDC, GetDeviceCaps, GetMonitorInfoW, LOGPIXELSX, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        MonitorFromPoint, ReleaseDC,
    },
    UI::WindowsAndMessaging::GetCursorPos,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitorSelector {
    MouseMonitor,
    PrimaryMonitor,
}

impl MonitorSelector {
    pub fn get_target_monitor_phy_rect(&self) -> Result<RECT> {
        unsafe {
            let target_cursor = match self {
                MonitorSelector::PrimaryMonitor => Ok(POINT { x: 0, y: 0 }),
                MonitorSelector::MouseMonitor => {
                    let mut point = std::mem::zeroed();
                    GetCursorPos(&mut point).map_or_else(
                        |e| Err(anyhow!("Failed to get cursor position: {e}")),
                        |_| Ok(point),
                    )
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
