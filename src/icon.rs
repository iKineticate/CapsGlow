use crate::theme::Theme;

use anyhow::Result;
use image::{ImageBuffer, ImageReader, Rgba};

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeIcon {
    Single(ImageBuffer<Rgba<u8>, Vec<u8>>),
    Theme {
        light: ImageBuffer<Rgba<u8>, Vec<u8>>,
        dark: ImageBuffer<Rgba<u8>, Vec<u8>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorIcons {
    pub icon: ThemeIcon,
    pub size: (u32, u32),
}

impl IndicatorIcons {
    pub fn find_icon_from_exe_dir() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;

        let icon_path = exe_dir.join("capslock.png");
        let icon_dark_path = exe_dir.join("capslock_dark.png");
        let icon_light_path = exe_dir.join("capslock_light.png");

        if icon_path.is_file() {
            let icon_date = ImageReader::open(icon_path)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();

            // 尚未决定是否需要确保图片大小控制在某范围
            let (width, height) = icon_date.dimensions();

            return Some(IndicatorIcons {
                icon: ThemeIcon::Single(icon_date),
                size: (width, height),
            });
        } else if icon_dark_path.is_file() && icon_light_path.is_file() {
            let icon_dark_date = ImageReader::open(icon_dark_path)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();
            let icon_light_date = ImageReader::open(icon_light_path)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();

            // 确保图片大小一致
            let (width, height) = icon_dark_date.dimensions();
            if icon_light_date.dimensions() != (width, height) {
                return None;
            }

            return Some(IndicatorIcons {
                icon: ThemeIcon::Theme {
                    light: icon_light_date,
                    dark: icon_dark_date,
                },
                size: (width, height),
            });
        }

        None
    }

    pub fn get_icon_date_and_size(
        &self,
        theme: &Theme,
    ) -> (ImageBuffer<Rgba<u8>, Vec<u8>>, (u32, u32)) {
        match &self.icon {
            ThemeIcon::Single(data) => (data.clone(), self.size),
            ThemeIcon::Theme { light, dark } => match theme {
                Theme::Light => (light.clone(), self.size),
                Theme::Dark => (dark.clone(), self.size),
            },
        }
    }
}

pub fn render_icon_to_buffer(
    buffer: &mut softbuffer::Buffer<
        '_,
        std::rc::Rc<winit::window::Window>,
        std::rc::Rc<winit::window::Window>,
    >,
    icon_buffer: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    icon_size: (u32, u32),
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<()> {
    buffer.fill(0);

    // 计算居中位置
    let start_x = (window_physical_width.saturating_sub(icon_size.0)) / 2;
    let start_y = (window_physical_height.saturating_sub(icon_size.1)) / 2;

    // 确保图标不会超出窗口边界
    let end_x = (start_x + icon_size.0).min(window_physical_width);
    let end_y = (start_y + icon_size.1).min(window_physical_height);
    let render_width = end_x.saturating_sub(start_x);
    let render_height = end_y.saturating_sub(start_y);

    // 遍历需要渲染的每个像素
    for y in 0..render_height {
        for x in 0..render_width {
            let pixel = icon_buffer.get_pixel(x, y).0;
            let a = pixel[3] as u32;

            // Alpha blending fix：预乘
            let alpha_f = a as f32 / 255.0;
            let r = (pixel[0] as f32 * alpha_f).round() as u32;
            let g = (pixel[1] as f32 * alpha_f).round() as u32;
            let b = (pixel[2] as f32 * alpha_f).round() as u32;

            let dst_x = start_x + x;
            let dst_y = start_y + y;

            if dst_x < window_physical_width && dst_y < window_physical_height {
                let idx = (dst_y * window_physical_height + dst_x) as usize;
                buffer[idx] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }

    Ok(())
}
