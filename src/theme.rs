use crate::{monitor::get_scale_factor, Theme, TEXT_PADDING, WINDOW_LOGICAL_SIZE};

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
    GetDIBits, GetDeviceCaps, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HORZRES, SRCCOPY, VERTRES,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const PERSONALIZE_REGISTRY_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS_USE_LIGHT_THEME_REGISTRY_KEY: &str = "AppsUseLightTheme";

pub fn get_windows_theme() -> Theme {
    let personalize_reg_key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(PERSONALIZE_REGISTRY_KEY, KEY_READ | KEY_WRITE)
        .expect("This program requires Windows 10 14393 or above");

    let theme_reg_value: u32 = personalize_reg_key
        .get_value(APPS_USE_LIGHT_THEME_REGISTRY_KEY)
        .expect("This program requires Windows 10 14393 or above");

    match theme_reg_value {
        0 => Theme::Dark,
        _ => Theme::Light,
    }
}

pub fn get_indicator_area_theme() -> Theme {
    unsafe {
        let hdc_screen = GetDC(None);

        let scale = get_scale_factor();
        let img_size = ((WINDOW_LOGICAL_SIZE - TEXT_PADDING) * scale) as i32;
        let screen_width = GetDeviceCaps(Some(hdc_screen), HORZRES);
        let screen_height = GetDeviceCaps(Some(hdc_screen), VERTRES);
        if screen_width < img_size || screen_height < img_size {
            return Theme::Light;
        }

        let x_start = (screen_width - img_size) / 2;
        let y_start = (screen_height - img_size) / 2;
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let h_bitmap = CreateCompatibleBitmap(hdc_screen, img_size, img_size);
        let _old_bitmap = SelectObject(hdc_mem, h_bitmap.into());

        // Capture the target area
        if let Err(_) = BitBlt(
            hdc_mem,
            0,
            0,
            img_size,
            img_size,
            Some(hdc_screen),
            x_start,
            y_start,
            SRCCOPY,
        ) {
            return Theme::Light;
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
            Theme::Light
        } else {
            Theme::Dark
        }
    }
}
