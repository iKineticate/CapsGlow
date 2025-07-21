use std::ops::DerefMut;

use crate::{Theme, ThemeDetectionSource};

use anyhow::{Result, anyhow};
use piet_common::{
    BitmapTarget, Color, Device, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

const WHITE: u32 = 0xffffffcc;
const BLACK: u32 = 0x1F1F1FCC;

pub fn render_font_to_sufface(
    buffer: &mut softbuffer::Buffer<
        '_,
        std::rc::Rc<winit::window::Window>,
        std::rc::Rc<winit::window::Window>,
    >,
    follow_system_theme: (ThemeDetectionSource, Theme),
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<()> {
    buffer.fill(0);

    let mut raw_pixels: Vec<u8> =
        vec![0; (window_physical_width * window_physical_height * 4) as usize]; // 每个像素4字节（RGBA）
    let mut device = Device::new().map_err(|e| anyhow!("Failed to get Device - {e}"))?;
    let mut bitmap_target = get_font_bitmap(
        &mut device,
        follow_system_theme,
        window_physical_width,
        window_physical_height,
    )?;
    bitmap_target
        .copy_raw_pixels(piet_common::ImageFormat::RgbaPremul, &mut raw_pixels)
        .map_err(|e| anyhow!("Failed to copy raw pixels - {e}"))?;
    let raw_pixels_u32: &[u32] = bytemuck::cast_slice(&raw_pixels);

    let surface_buffer: &mut [u32] = buffer.deref_mut();
    surface_buffer[..raw_pixels_u32.len()].copy_from_slice(raw_pixels_u32);

    Ok(())
}

fn get_font_bitmap(
    device: &mut Device,
    follow_system_theme: (ThemeDetectionSource, Theme),
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<BitmapTarget<'_>> {
    let mut bitmap_target = device
        .bitmap_target(
            window_physical_width as usize,
            window_physical_height as usize,
            1.0,
        )
        .map_err(|e| anyhow!("Failed to create a new bitmap target. - {e}"))?;

    let mut piet = bitmap_target.render_context();
    piet.clear(None, Color::TRANSPARENT);

    let color = if follow_system_theme.1 == Theme::Dark {
        WHITE
    } else {
        BLACK
    };

    // Dynamically calculated font size
    let mut layout;
    let mut font_size = 10.0;
    let text = piet.text();
    loop {
        layout = text
            .new_text_layout("🔒")
            .font(FontFamily::new_unchecked("Arial"), font_size)
            .text_color(Color::from_rgba32_u32(color)) // 0xffffff + alpha:00~ff
            .build()
            .map_err(|e| anyhow!("Failed to build text layout - {e}"))?;

        if layout.size().width > window_physical_width as f64
            || layout.size().height > window_physical_height as f64
        {
            break;
        }
        font_size += 1.0;
    }

    let (x, y) = (
        (window_physical_width as f64 - layout.size().width) / 2.0,
        (window_physical_height as f64 - layout.size().height) / 2.0,
    );

    piet.draw_text(&layout, (x, y));
    piet.finish().map_err(|e| anyhow!("{e}"))?;
    drop(piet);

    Ok(bitmap_target)
}
