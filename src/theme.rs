use crate::util::to_wide;

use image::Rgba;
use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap,
            CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits,
            GetDeviceCaps, HORZRES, SRCCOPY, SelectObject, VERTRES,
        },
        System::Registry::{HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD, RegGetValueW},
    },
    core::PCWSTR,
};

const PERSONALIZE_REGISTRY_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const SYSTEM_USES_LIGHT_THEME_REGISTRY_KEY: &str = "SystemUsesLightTheme";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemTheme {
    Light,
    Dark,
}

impl SystemTheme {
    fn get() -> Self {
        let path = to_wide(PERSONALIZE_REGISTRY_KEY);
        let name = to_wide(SYSTEM_USES_LIGHT_THEME_REGISTRY_KEY);

        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let mut reg_dword = REG_DWORD;

        let ret = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_DWORD,
                Some(&mut reg_dword),
                Some(&mut value as *mut _ as *mut _),
                Some(&mut size as *mut _),
            )
        };

        if ret.is_err() {
            SystemTheme::Light
        } else {
            match value {
                0 => SystemTheme::Dark,
                _ => SystemTheme::Light,
            }
        }
    }

    pub fn get_font_color(&self) -> Rgba<u8> {
        match self {
            Self::Dark => Rgba([255, 255, 255, 255]),
            Self::Light => Rgba([31, 31, 31, 255]),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IndicatorTheme {
    System,
    #[default]
    IndicatorArea,
}

impl IndicatorTheme {
    pub fn get_theme(&self, scale: f64, window_phy_size: f64) -> SystemTheme {
        match self {
            IndicatorTheme::System => SystemTheme::get(),
            IndicatorTheme::IndicatorArea => Self::get_indicator_area_theme(scale, window_phy_size),
        }
    }

    pub fn get_indicator_area_theme(scale: f64, window_phy_size: f64) -> SystemTheme {
        unsafe {
            let hdc_screen = GetDC(None);

            let img_size = (window_phy_size * scale) as i32;
            let screen_width = GetDeviceCaps(Some(hdc_screen), HORZRES);
            let screen_height = GetDeviceCaps(Some(hdc_screen), VERTRES);
            if screen_width < img_size || screen_height < img_size {
                return SystemTheme::Light;
            }

            let x_start = (screen_width - img_size) / 2;
            let y_start = (screen_height - img_size) / 2;
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            let h_bitmap = CreateCompatibleBitmap(hdc_screen, img_size, img_size);
            let _old_bitmap = SelectObject(hdc_mem, h_bitmap.into());

            // Capture the target area
            if BitBlt(
                hdc_mem,
                0,
                0,
                img_size,
                img_size,
                Some(hdc_screen),
                x_start,
                y_start,
                SRCCOPY,
            )
            .is_err()
            {
                return SystemTheme::Light;
            };

            // Preparing the BITMAPINFO structure
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: img_size,
                    biHeight: -img_size, // 从上到下排列
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut buffer = vec![0u8; (img_size * img_size * 4) as usize];
            GetDIBits(
                hdc_mem,
                h_bitmap,
                0,
                img_size as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut bmi as *mut _ as *mut _,
                DIB_RGB_COLORS,
            );

            let total_brightness: f32 = buffer
                .chunks_exact(4)
                .map(|chunk| {
                    // 注意Windows的GDI返回BGR格式
                    let r = chunk[2] as f32;
                    let g = chunk[1] as f32;
                    let b = chunk[0] as f32;
                    0.2126 * r + 0.7152 * g + 0.0722 * b // 亮度计算公式
                })
                .sum();

            let avg = total_brightness / (img_size * img_size * 255) as f32;

            DeleteObject(h_bitmap.into()).unwrap();
            DeleteDC(hdc_mem).unwrap();
            DeleteDC(hdc_screen).unwrap();

            if avg > 0.5 {
                SystemTheme::Light
            } else {
                SystemTheme::Dark
            }
        }
    }
}
