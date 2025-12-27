use std::{path::PathBuf, sync::LazyLock};

use ab_glyph::{Font, FontVec, Glyph, Point, PxScale};
use anyhow::{Context, Result, anyhow};
use image::{ImageBuffer, ImageReader, Rgba};

use crate::{config::EXE_PATH, theme::SystemTheme};

pub const LOGO_DATA: &[u8] = include_bytes!("../assets/logo.ico");

pub static INDICATOR_ICON_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| EXE_PATH.with_file_name("capslock.png"));

pub static INDICATOR_ICON_DARK_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| EXE_PATH.with_file_name("capslock_dark.png"));

pub static INDICATOR_ICON_LIGHT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| EXE_PATH.with_file_name("capslock_light.png"));

#[derive(Debug, Clone, PartialEq)]
enum IconDate {
    Normal(ImageBuffer<Rgba<u8>, Vec<u8>>),
    Theme {
        light: ImageBuffer<Rgba<u8>, Vec<u8>>,
        dark: ImageBuffer<Rgba<u8>, Vec<u8>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomIcon {
    icon: IconDate,
    size: (u32, u32),
}

impl CustomIcon {
    pub fn find_custom_icon() -> Option<Self> {
        if INDICATOR_ICON_PATH.is_file() {
            let icon_date = ImageReader::open(&*INDICATOR_ICON_PATH)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();

            let (width, height) = icon_date.dimensions();

            Some(CustomIcon {
                icon: IconDate::Normal(icon_date),
                size: (width, height),
            })
        } else if INDICATOR_ICON_DARK_PATH.is_file() && INDICATOR_ICON_LIGHT_PATH.is_file() {
            let icon_dark_date = ImageReader::open(&*INDICATOR_ICON_DARK_PATH)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();

            let icon_light_date = ImageReader::open(&*INDICATOR_ICON_LIGHT_PATH)
                .ok()?
                .decode()
                .ok()?
                .into_rgba8();

            // 确保深浅色图标大小一致
            let (width, height) = icon_dark_date.dimensions();
            if icon_light_date.dimensions() != (width, height) {
                log::error!("Icon size mismatch between light and dark themes.");
                None
            } else {
                Some(CustomIcon {
                    icon: IconDate::Theme {
                        light: icon_light_date,
                        dark: icon_dark_date,
                    },
                    size: (width, height),
                })
            }
        } else {
            None
        }
    }

    pub fn get_icon_date_and_size(
        &self,
        theme: SystemTheme,
    ) -> (ImageBuffer<Rgba<u8>, Vec<u8>>, (u32, u32)) {
        match &self.icon {
            IconDate::Normal(data) => (data.clone(), self.size),
            IconDate::Theme { light, dark } => match theme {
                SystemTheme::Light => (light.clone(), self.size),
                SystemTheme::Dark => (dark.clone(), self.size),
            },
        }
    }

    pub fn get_size(&self) -> (u32, u32) {
        log::info!("Custom icon size: {:?}", self.size);
        self.size
    }
}

pub fn render_font_to_sufface(
    buffer: &mut softbuffer::Buffer<
        '_,
        std::rc::Rc<winit::window::Window>,
        std::rc::Rc<winit::window::Window>,
    >,
    color: Rgba<u8>,
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<()> {
    let font_path = r"C:\WINDOWS\FONTS\SEGUIEMJ.TTF";
    let font_data = std::fs::read(font_path)?;
    let font = FontVec::try_from_vec(font_data).context("Failed to parse font")?;

    let base_scale = PxScale::from(100.0); // 任意较大的基准值

    let glyph_id = font.glyph_id('\u{1F512}');
    let glyph = glyph_id.with_scale(base_scale);
    let outlined = font.outline_glyph(glyph).unwrap();
    let bounds = outlined.px_bounds();

    let window_width = window_physical_width as f32;
    let window_height = window_physical_height as f32;

    let factor = f32::min(
        window_width / bounds.width(),
        window_height / bounds.height(),
    );

    let final_scale = PxScale {
        x: base_scale.x * factor,
        y: base_scale.y * factor,
    };

    let glyph_for_bounds = glyph_id.with_scale(final_scale);
    let outlined = font.outline_glyph(glyph_for_bounds).unwrap();
    let final_bounds = outlined.px_bounds();

    let position = Point {
        x: (window_width - final_bounds.width()) / 2.0 - final_bounds.min.x,
        y: (window_height - final_bounds.height()) / 2.0 - final_bounds.min.y,
    };

    let glyph = Glyph {
        id: glyph_id,
        scale: final_scale,
        position,
    };

    let sr = color[0] as f32 / 255.0;
    let sg = color[1] as f32 / 255.0;
    let sb = color[2] as f32 / 255.0;
    let sa = color[3] as f32 / 255.0;

    let stride = u32::from(buffer.width());

    if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();
        let start_x = bounds.min.x as i32;
        let start_y = bounds.min.y as i32;

        outlined.draw(|x, y, coverage| {
            // 计算在 buffer 中的绝对坐标
            // x, y 是相对于 bounds.min 的偏移
            let screen_x = start_x + x as i32;
            let screen_y = start_y + y as i32;

            if screen_x < 0
                || screen_x >= window_physical_width as i32
                || screen_y < 0
                || screen_y >= window_physical_height as i32
            {
                return;
            }

            let out_a = coverage * sa;
            if out_a <= 0.0 {
                return;
            }

            let r = (sr * out_a * 255.0) as u32;
            let g = (sg * out_a * 255.0) as u32;
            let b = (sb * out_a * 255.0) as u32;
            let a = (out_a * 255.0) as u32;

            let idx = (screen_y as u32 * stride + screen_x as u32) as usize;
            buffer[idx] = (a << 24) | (r << 16) | (g << 8) | b;
        });
    }

    Ok(())
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
    let stride = u32::from(buffer.width());

    // 计算居中位置
    let start_x = (window_physical_width.saturating_sub(icon_size.0)) / 2;
    let start_y = (window_physical_height.saturating_sub(icon_size.1)) / 2;

    // 确保图标不会超出窗口边界
    let render_width = icon_size
        .0
        .min(window_physical_width.saturating_sub(start_x));
    let render_height = icon_size
        .1
        .min(window_physical_height.saturating_sub(start_y));

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

            // 修正后的索引计算：y * 宽度 + x
            let idx = (dst_y * stride + dst_x) as usize;

            // 写入 buffer
            if idx < buffer.len() {
                buffer[idx] = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }

    Ok(())
}

pub fn load_icon_for_tray() -> Result<tray_icon::Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(LOGO_DATA)
            .map_err(|e| anyhow!("Failed to load icon - {e}"))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .with_context(|| "Failed to crate the logo")
}

pub fn load_icon_for_window() -> Result<winit::window::Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(LOGO_DATA)
            .map_err(|e| anyhow!("Failed to load icon - {e}"))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .with_context(|| "Failed to crate the logo")
}
