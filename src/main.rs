use std::{num::NonZeroU32, rc::Rc};

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowLongPtrW, SetLayeredWindowAttributes,
    LAYERED_WINDOW_ATTRIBUTES_FLAGS,
    GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};
use windows::Win32::Foundation::HWND;
use piet_common::{Color, Device, RenderContext, Text, FontFamily, TextLayout, TextLayoutBuilder, };

#[allow(clippy::single_match)]
fn main() {
    let event_loop = EventLoop::new();

    // 获取鼠标所在的屏幕大小


    let window = WindowBuilder::new()
        .with_title("CapsLock")
        .with_undecorated_shadow(false)
        .with_always_on_top(true)
        // .with_position()
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    unsafe {
        let hwnd = HWND(window.hwnd() as _);
        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style.0 as isize);
        SetLayeredWindowAttributes(
            hwnd, 
            windows::Win32::Foundation::COLORREF(0), /* crKey */
            255, /* Alpha: 0 ~ 255 */
            LAYERED_WINDOW_ATTRIBUTES_FLAGS(0x00000002) /* LWA_ALPHA: 0x00000002(窗口透明), LWA_COLORKEY: 0x0x00000001(指定crKey颜色透明) */
        ).unwrap();
    }

    let (window, _context, mut surface) = {
        let window = Rc::new(window);
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
        (window, context, surface)
    };

    let mut device = Device::new().unwrap();
    // let mut last_caps_state = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::RedrawRequested(_) => {
                // let caps_state = unsafe { (GetKeyState(0x14) & 0x0001) != 0 };

                // if caps_state != last_caps_state {

                // }

                let (width, height) = (
                    window.inner_size().width as usize,
                    window.inner_size().height as usize
                );

                let caps_state = true;

                let mut bitmap_target = match device.bitmap_target(width, height, 1.0) {
                    Ok(t) => t,
                    Err(_) => return,
                };
                let mut piet = bitmap_target.render_context();
                piet.clear(None, Color::TRANSPARENT);

                if caps_state {
                    let text = piet.text();
                    let layout = text.new_text_layout("A")
                        .font(FontFamily::new_unchecked("Arial".to_string()), 72.0)
                        .text_color(Color::rgba(1.0, 1.0, 1.0, 1.0))
                        .build()
                        .unwrap();

                    let (x, y) = (
                        (width as f64 - layout.size().width) / 2.0,
                        (height as f64 - layout.size().height) / 2.0
                    );

                    piet.draw_text(&layout, (x, y));
                }

                // Drop the first mutable borrow before the second one
                piet.finish().unwrap();
                drop(piet);

                surface
                    .resize(
                        NonZeroU32::new(width as u32).unwrap(),
                        NonZeroU32::new(height as u32).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();
                let buffer_slice = buffer.as_mut();
                let buffer_slice_u8 = bytemuck::cast_slice_mut(buffer_slice);
                bitmap_target.copy_raw_pixels(piet_common::ImageFormat::RgbaPremul, buffer_slice_u8).unwrap();

                buffer.present().unwrap();
            }

            _ => (),
        }
    });
}
