use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use ab_glyph::{Font, FontVec, Glyph, Point, PxScale};
use anyhow::{Context, Result, anyhow};
use image::{GenericImageView, Rgba};
use serde::{Deserialize, Serialize};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWriteCreateFactory,
    IDWriteFactory, IDWriteLocalFontFileLoader,
};
use windows::core::{BOOL, HSTRING, Interface};

use crate::config::EXE_DIR;

const LOGO_DATA: &[u8] = include_bytes!("../assets/logo.ico");
const ICON_NORMAL: &str = "capslock.png";
const ICON_LIGHT: &str = "capslock-light.png";
const ICON_DARK: &str = "capslock-dark.png";

static FONT_SEGOE_FLUENT_PATH: &str = r"C:\WINDOWS\FONTS\SEGOEICONS.TTF";
static FONT_SEGOE_MDL2_PATH: &str = r"C:\WINDOWS\FONTS\SEGMDL2.TTF";
pub static FONT_DEFAULT_PATH: LazyLock<String> = LazyLock::new(|| {
    // Win11 使用 [Segoe Fluent Icons] 字体
    // Win10 使用 [Segoe MDL2 Assets] 字体
    if !Path::new(FONT_SEGOE_FLUENT_PATH).is_file() {
        check_font_exists("Segoe Fluent Icons")
            .or_else(|| check_font_exists("SegoeFluentIcons"))
            .or_else(|| check_font_exists("SEGOEICONS"))
            .unwrap_or(FONT_SEGOE_MDL2_PATH.to_owned())
    } else {
        FONT_SEGOE_FLUENT_PATH.to_owned()
    }
});


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndicatorIcon {
    Font { glyph: char, font_path: String },
    Custom(CustomIcon),
}

impl Default for IndicatorIcon {
    fn default() -> Self {
        let light = EXE_DIR.join(ICON_LIGHT);
        let dark = EXE_DIR.join(ICON_DARK);
        if light.is_file() && dark.is_file() {
            return Self::Custom(CustomIcon::Theme { light, dark });
        }

        if light.is_file() || dark.is_file() {
            log::warn!(
                "Only one side's theme icon was found; Theme requires both light and dark to exist."
            );
        }

        let normal = EXE_DIR.join(ICON_NORMAL);
        if normal.is_file() {
            return Self::Custom(CustomIcon::Normal(normal));
        }

        IndicatorIcon::Font {
            glyph: '\u{E72E}',
            font_path: FONT_DEFAULT_PATH.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CustomIcon {
    Normal(PathBuf),
    Theme { light: PathBuf, dark: PathBuf },
}

pub fn render_font_to_surface(
    buffer: &mut softbuffer::Buffer<
        '_,
        std::rc::Rc<winit::window::Window>,
        std::rc::Rc<winit::window::Window>,
    >,
    glyph: char,
    font_color: Rgba<u8>,
    font_path: String,
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<()> {
    log::info!(
        "{glyph} - {font_path} - {font_color:?} - {window_physical_width}x{window_physical_height}"
    );
    let font_data = std::fs::read(font_path)?;
    let font =
        FontVec::try_from_vec(font_data).map_err(|e| anyhow!("failed to parse font - {e}"))?;

    let base_scale = PxScale::from(100.0); // 任意较大的基准值

    let glyph_id = font.glyph_id(glyph);
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

    let sr = font_color[0] as f32 / 255.0;
    let sg = font_color[1] as f32 / 255.0;
    let sb = font_color[2] as f32 / 255.0;
    let sa = font_color[3] as f32 / 255.0;

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
    image: image::DynamicImage,
    window_physical_width: u32,
    window_physical_height: u32,
) -> Result<()> {
    // let stride = u32::from(buffer.width());

    // // 计算居中位置
    // let start_x = (window_physical_width.saturating_sub(icon_size.0)) / 2;
    // let start_y = (window_physical_height.saturating_sub(icon_size.1)) / 2;

    // // 确保图标不会超出窗口边界
    // let render_width = icon_size
    //     .0
    //     .min(window_physical_width.saturating_sub(start_x));
    // let render_height = icon_size
    //     .1
    //     .min(window_physical_height.saturating_sub(start_y));

    // // 遍历需要渲染的每个像素
    // for y in 0..render_height {
    //     for x in 0..render_width {
    //         let pixel = icon_buffer.get_pixel(x, y).0;
    //         let a = pixel[3] as u32;

    //         // Alpha blending fix：预乘
    //         let alpha_f = a as f32 / 255.0;
    //         let r = (pixel[0] as f32 * alpha_f).round() as u32;
    //         let g = (pixel[1] as f32 * alpha_f).round() as u32;
    //         let b = (pixel[2] as f32 * alpha_f).round() as u32;

    //         let dst_x = start_x + x;
    //         let dst_y = start_y + y;

    //         // 修正后的索引计算：y * 宽度 + x
    //         let idx = (dst_y * stride + dst_x) as usize;

    //         // 写入 buffer
    //         if idx < buffer.len() {
    //             buffer[idx] = (a << 24) | (r << 16) | (g << 8) | b;
    //         }
    //     }
    // }

    // Ok(())
    let dst_w = window_physical_width as usize;
    let dst_h = window_physical_height as usize;
    if dst_w == 0 || dst_h == 0 {
        return Ok(());
    }

    let dst = &mut buffer[..];

    let (src_w, src_h) = image.dimensions();
    let (src_w, src_h) = (src_w as usize, src_h as usize);
    if src_w == 0 || src_h == 0 {
        return Ok(());
    }
    let src = image.as_bytes();
    let src_stride = src_w * 4;

    if src_w == dst_w && src_h == dst_h {
        // 快速路径：逐行 zip，内层无边界检查，编译器可向量化
        for (drow, srow) in dst
            .chunks_exact_mut(dst_w)
            .zip(src.chunks_exact(src_stride))
        {
            for (d, s) in drow.iter_mut().zip(srow.chunks_exact(4)) {
                blend_px(d, s[0], s[1], s[2], s[3]);
            }
        }
    } else {
        // 缩放路径：预计算每列的源坐标和权重，行内复用
        let mut xmap = Vec::with_capacity(dst_w);
        for dx in 0..dst_w {
            let fx = ((dx as f32 + 0.5) * src_w as f32 / dst_w as f32 - 0.5)
                .clamp(0.0, (src_w - 1) as f32);
            let x0 = fx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            xmap.push((x0, x1, fx - x0 as f32));
        }

        for dy in 0..dst_h {
            let fy = ((dy as f32 + 0.5) * src_h as f32 / dst_h as f32 - 0.5)
                .clamp(0.0, (src_h - 1) as f32);
            let y0 = fy.floor() as usize;
            let y1 = (y0 + 1).min(src_h - 1);
            let ty = fy - y0 as f32;

            let row0 = &src[y0 * src_stride..y0 * src_stride + src_stride];
            let row1 = &src[y1 * src_stride..y1 * src_stride + src_stride];

            let drow = &mut dst[dy * dst_w..dy * dst_w + dst_w];
            for (dx, d) in drow.iter_mut().enumerate() {
                let (x0, x1, tx) = xmap[dx];
                let px = bilerp(
                    &row0[x0 * 4..x0 * 4 + 4],
                    &row0[x1 * 4..x1 * 4 + 4],
                    &row1[x0 * 4..x0 * 4 + 4],
                    &row1[x1 * 4..x1 * 4 + 4],
                    tx,
                    ty,
                );
                blend_px(d, px[0], px[1], px[2], px[3]);
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

#[inline(always)]
fn div255(x: u32) -> u32 {
    // round(x / 255)，x <= 255*255，精确
    let x = x + 128;
    (x + (x >> 8)) >> 8
}

#[inline(always)]
fn blend_px(dst: &mut u32, r: u8, g: u8, b: u8, a: u8) {
    match a {
        0 => {} // 全透明，保留背景
        255 => {
            // 不透明：直接覆盖，高 8 位保持 0
            *dst = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        }
        _ => {
            let a = a as u32;
            let inv = 255 - a;
            let dr = (*dst >> 16) & 0xff;
            let dg = (*dst >> 8) & 0xff;
            let db = *dst & 0xff;
            let nr = div255(r as u32 * a + dr * inv);
            let ng = div255(g as u32 * a + dg * inv);
            let nb = div255(b as u32 * a + db * inv);
            *dst = (nr << 16) | (ng << 8) | nb;
        }
    }
}

#[inline(always)]
fn bilerp(p00: &[u8], p01: &[u8], p10: &[u8], p11: &[u8], tx: f32, ty: f32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = p00[i] as f32 * (1.0 - tx) + p01[i] as f32 * tx;
        let bot = p10[i] as f32 * (1.0 - tx) + p11[i] as f32 * tx;
        let v = top * (1.0 - ty) + bot * ty;
        out[i] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

pub fn check_font_exists(input: &str) -> Option<String> {
    let input = input.trim();
    let input_as_path = Path::new(input);
    let input_is_font_type = input_as_path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "ttf" | "otf"));

    if input_as_path.is_file() && input_is_font_type {
        return Some(input.to_string());
    }

    let famaily_name = input_is_font_type
        .then(|| input_as_path.file_stem().and_then(|s| s.to_str()))
        .flatten()
        .unwrap_or(input);

    let file_name = if input_is_font_type {
        input.to_string()
    } else {
        format!("{input}.ttf")
    };

    resolve_font_paths(famaily_name, None, None)
        .inspect_err(|e| log::error!("failed to get [{famaily_name}] font path: {e}"))
        .ok()
        .and_then(|paths| paths.into_iter().next())
        .filter(|path| Path::new(path).is_file())
        .or_else(|| {
            std::env::var_os("WINDIR").and_then(|windir| {
                let path = PathBuf::from(windir).join("Fonts").join(&file_name);
                path.is_file().then_some(path.to_string_lossy().to_string())
            })
        })
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA").and_then(|local_appdata| {
                let path = PathBuf::from(local_appdata)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts")
                    .join(file_name);
                path.is_file().then_some(path.to_string_lossy().to_string())
            })
        })
}

fn resolve_font_paths(
    family_name: &str,
    weight: Option<DWRITE_FONT_WEIGHT>,
    style: Option<DWRITE_FONT_STYLE>,
) -> Result<Vec<String>> {
    unsafe {
        let factory = DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut font_collection = None;
        factory.GetSystemFontCollection(&mut font_collection, false)?;

        let Some(font_collection) = font_collection else {
            return Ok(vec![]);
        };

        let mut index = 0;
        let mut exists = BOOL::default();
        font_collection.FindFamilyName(&HSTRING::from(family_name), &mut index, &mut exists)?;

        if !exists.as_bool() {
            return Ok(vec![]);
        }

        let font_family = font_collection.GetFontFamily(index)?;

        let font = font_family.GetFirstMatchingFont(
            weight.unwrap_or(DWRITE_FONT_WEIGHT_NORMAL),
            DWRITE_FONT_STRETCH_NORMAL,
            style.unwrap_or(DWRITE_FONT_STYLE_NORMAL),
        )?;

        let font_face = font.CreateFontFace()?;

        let mut file_count = 0;
        font_face.GetFiles(&mut file_count, None)?;

        if file_count == 0 {
            return Ok(vec![]);
        }

        let mut font_files = vec![None; file_count as usize];
        font_face.GetFiles(&mut file_count, Some(font_files.as_mut_ptr()))?;

        let mut result = Vec::new();
        for file in font_files.iter().flatten() {
            let loader = file.GetLoader()?;
            let Ok(local_loader) = loader.cast::<IDWriteLocalFontFileLoader>() else {
                continue;
            };
            let mut key: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut key_size = 0;

            file.GetReferenceKey(&mut key, &mut key_size)?;

            let mut font_file_path_length = local_loader.GetFilePathLengthFromKey(key, key_size)?;

            // returned length does not include NULL-terminator
            // (https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritelocalfontfileloader-getfilepathlengthfromkey)
            font_file_path_length += 1;

            let mut font_file_path_buffer = vec![0u16; font_file_path_length as usize];

            local_loader.GetFilePathFromKey(key, key_size, &mut font_file_path_buffer)?;

            font_file_path_buffer.set_len(font_file_path_length as usize);

            // remove null terminator !!!
            let path = String::from_utf16_lossy(&font_file_path_buffer)
                .trim_end_matches('\0')
                .trim()
                .to_string();

            result.push(path);
        }

        Ok(result)
    }
}
