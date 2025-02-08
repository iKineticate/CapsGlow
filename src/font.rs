use anyhow::{anyhow, Result};
use piet_common::{
    BitmapTarget, Color, Device, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

pub fn get_font_bitmap<'a>(
    device: &'a mut Device,
    width: u32,
    height: u32,
    text_padding: f64,
    scale: f64,
) -> Result<BitmapTarget<'a>> {
    let mut bitmap_target = device
        .bitmap_target(width as usize, height as usize, 1.0)
        .map_err(|e| anyhow!("Failed to create a new bitmap target. - {e}"))?;
    let mut piet = bitmap_target.render_context();
    piet.clear(None, Color::TRANSPARENT);

    // Dynamically calculated font size
    let mut layout;
    let mut font_size = 10.0;
    let text = piet.text();
    loop {
        layout = text
            .new_text_layout("🔒")
            .font(FontFamily::new_unchecked("Arial"), font_size)
            .text_color(Color::from_rgba32_u32(0xffffffcc)) // 0xffffff + alpha:00~ff
            .build()
            .map_err(|e| anyhow!("Failed to build text layout - {e}"))?;

        if layout.size().width > width as f64 - text_padding * scale
            || layout.size().height > height as f64 - text_padding * scale
        {
            break;
        }
        font_size += 1.0;
    }

    let (x, y) = (
        (width as f64 - layout.size().width) / 2.0,
        (height as f64 - layout.size().height) / 2.0,
    );

    // Drop the first mutable borrow before the second one
    piet.draw_text(&layout, (x, y));
    piet.finish()
        .map_err(|e| anyhow!("Failed to finish drawing - {e}"))?;
    drop(piet);

    Ok(bitmap_target)
}
