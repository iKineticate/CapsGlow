use anyhow::Result;
use windows::Win32::{
    Foundation::POINT,
    Graphics::Gdi::{
        GetDC, GetDeviceCaps, GetMonitorInfoW, MonitorFromPoint, ReleaseDC, LOGPIXELSX,
        MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    },
};

pub fn get_primary_monitor_logical_size(scale: f64) -> Result<(f64, f64)> {
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        GetMonitorInfoW(monitor, &mut info).ok()?;

        Ok((
            (info.rcMonitor.right - info.rcMonitor.left) as f64 / scale,
            (info.rcMonitor.bottom - info.rcMonitor.top) as f64 / scale,
        ))
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
