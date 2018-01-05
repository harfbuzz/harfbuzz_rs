extern crate harfbuzz_rs;
extern crate rusttype;
extern crate image;
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate graphics;
extern crate gfx_graphics;
extern crate shader_version;

use graphics::{Graphics, Viewport, clear, Image};
use graphics::context::Context;
use gfx_graphics::{Gfx2d, Texture, TextureSettings};
use gfx::Device;
use glutin::{GlRequest, Api};
use shader_version::OpenGL;
use image::{Rgba, RgbaImage};

use harfbuzz_rs::{Font, Face, UnicodeBuffer, HbArc};
use harfbuzz_rs::rusttype::{ScaledRusttypeFont, RT_FONT_FUNCS};

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

fn main() {
    let events_loop = glutin::EventsLoop::new();
    let builder = glutin::WindowBuilder::new()
        .with_title("HarfBuzz example".to_string())
        .with_srgb(Some(true))
        .with_vsync()
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 2)));

    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    unsafe {
        window.make_current().unwrap();
    }

    let mut g2d = Gfx2d::new(OpenGL::V3_2, &mut factory);

    let glyphs = shape(&window, &mut factory);

    let mut running = true;
    events_loop.run_forever(|glutin::Event::WindowEvent{window_id: _, event}| {
        if !running {
            // don't process any leftover events after window is closed
            return
        }
        match event {
            glutin::WindowEvent::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape), _) |
            glutin::WindowEvent::Closed => {
                running = false;
                events_loop.interrupt();
            },
            glutin::WindowEvent::Resized(_width, _height) => {
                gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
            },
            _ => {},
        }

        // draw a frame
        let (width, height) = window.get_inner_size().unwrap_or((0, 0));
        let (draw_width, draw_height) = window.get_inner_size_pixels().unwrap_or((0, 0));
        let vp = Viewport {
            rect: [0, 0, draw_width as i32, draw_height as i32],
            draw_size: [draw_width, draw_height],
            window_size: [width, height],
        };
        g2d.draw(&mut encoder, &main_color, &main_depth, vp, |c, g| {
            draw_text(&c, g, &glyphs);
        });
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    });
}

fn draw_text<G: Graphics>(c: &Context, g: &mut G, content: &[(Image, G::Texture)]) {
    clear([1.0, 1.0, 1.0, 1.0], g);
    for &(image, ref texture) in content {
        image.draw(texture, &c.draw_state, c.transform, g);
    }
}

fn shape<F, R>(win: &glutin::Window, factory: &mut F) -> Vec<(Image, Texture<R>)>
    where F: gfx::Factory<R>,
          R: gfx::Resources {
    let index = 0;
    let path = "testfiles/DejaVuSans.ttf";
    let rt_font; // declare here so that there are no lifetime issues.
    let face = HbArc::from(Face::from_file(path, index).expect("Error reading font file."));
    let hb_font = Font::new(face);
    let mut hb_font = Font::create_sub_font(hb_font);


    let s_factor: f64 = 1f64 / win.hidpi_factor() as f64;
    let fontsize: f64 = 80.0;
    hb_font.set_scale(fontsize as i32 * 64, fontsize as i32 * 64);

    rt_font = ScaledRusttypeFont::from_hb_font(&hb_font);
    // hb_font.set_font_funcs(RT_FONT_FUNCS.clone(), &rt_font);
    let scale = rusttype::Scale {
        x: rt_font.scale.x / 64f32,
        y: rt_font.scale.y / 64f32,
    };

    // Create a buffer with some text and shape it...
    let result = UnicodeBuffer::new()
        .add_str("صِف خَلقَ خَودِ كَمِثلِ الشَمسِ")
        // .add_str("iiiiiiiillllllllll")
        .shape(&hb_font, &[]);

    // ... and get the results.
    let positions = result.get_glyph_positions();
    let infos = result.get_glyph_infos();

    let mut cursor = (10 * 64, 100 * 64);
    let mut vec = Vec::with_capacity(result.len());
    // iterate over the shaped glyphs
    for (position, info) in positions.iter().zip(infos) {
        let gid = info.codepoint;
        let x_advance = position.x_advance;
        let x_offset = position.x_offset;
        let y_offset = position.y_offset;

        let x_pos: f64 = (cursor.0 + x_offset) as f64 / 64.0;
        let y_pos: f64 = (cursor.1 + y_offset) as f64 / 64.0;

        // Separate x_pos and y_pos into integral and fractional parts for subpixel positioning.
        let x_pos_int = x_pos.trunc();
        let x_pos_frac = x_pos.fract();
        let y_pos_int = y_pos.trunc();
        let y_pos_frac = y_pos.fract();

        // We don't do metrics hinting, so no rounding is applied to the x_advance value.
        cursor.0 += x_advance;

        let glyph = match rt_font.font.glyph(rusttype::GlyphId(gid)) {
            Some(glyph) => glyph,
            None => continue,
        };

        let glyph = glyph.scaled(scale).positioned(rusttype::Point {
                                                       x: x_pos_frac as f32,
                                                       y: y_pos_frac as f32,
                                                   });
        let bbox = match glyph.pixel_bounding_box() {
            Some(bbox) => bbox,
            None => continue,
        };
        let image_buffer = draw_glyph(&glyph);

        let x_origin = (x_pos_int + bbox.min.x as f64) * s_factor;
        let y_origin = (y_pos_int + bbox.min.y as f64) * s_factor;
        let width = (bbox.max.x - bbox.min.x) as f64 * s_factor;
        let height = (bbox.max.y - bbox.min.y) as f64 * s_factor;
        let image = Image::new().rect([x_origin, y_origin, width, height]);

        let texture = Texture::from_image(factory, &image_buffer, &TextureSettings::new())
            .unwrap();

        vec.push((image, texture));
    }

    vec
}

fn draw_glyph(glyph: &rusttype::PositionedGlyph) -> RgbaImage {
    let bbox = glyph.pixel_bounding_box().expect("Glyph has no bounding box.");
    let mut image_buffer = RgbaImage::new((bbox.max.x - bbox.min.x) as u32,
                                          (bbox.max.y - bbox.min.y) as u32);
    glyph.draw(|x, y, v| {
                   let v = (255.0 * v).round() as u8;
                   image_buffer.put_pixel(x, y, Rgba { data: [0, 0, 0, v] })
               });
    image_buffer
}
