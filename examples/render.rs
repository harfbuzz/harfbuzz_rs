extern crate piston_window;
extern crate harfbuzz_rs;
extern crate image as piston_image;
extern crate rusttype;
extern crate glfw_window;

use harfbuzz_rs::{Face, UnicodeBuffer};
use harfbuzz_rs::rusttype::ScaledRusttypeFont;

use piston_window::*;
use piston_image::{ImageBuffer, Rgba};
use glfw_window::GlfwWindow;

fn main() {
    let mut window: PistonWindow<GlfwWindow> = WindowSettings::new("Hello World!", [512; 2])
        .build()
        .unwrap();

    window.set_swap_buffers(true);
    let glyphs = shape(&mut window);

    println!("window size {:?}", window.window.size());
    println!("window draw size {:?}", window.draw_size());
    println!("render target view dimensions {:?}",
             window.output_color.get_dimensions());

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            clear([1.0, 1.0, 1.0, 1.0], g);
            for &(image, ref texture) in &glyphs {
                image.draw(texture, &DrawState::default(), c.transform, g);
            }
        });
    }
}

fn shape<T: Window>(win: &mut PistonWindow<T>) -> Vec<(Image, G2dTexture)> {
    let index = 0;
    let path = "testfiles/Optima.ttc";
    let mut hb_font = Face::from_file(path, index)
        .expect("Error reading font file.")
        .create_font();


    let s_factor: f64 = 0.5;
    let fontsize: f64 = 80.0;
    hb_font.set_scale(fontsize as i32 * 64, fontsize as i32 * 64);
    hb_font.set_ppem(96 * 64, 96 * 64);

    let rt_font = ScaledRusttypeFont::from_hb_font(&hb_font);

    // Create a buffer with some text and shape it...
    let result = UnicodeBuffer::new().add_str("Hello World! iiiiiiiiiiiiiiiiii").shape(&hb_font, &[]);

    // ... and get the results.
    let positions = result.get_glyph_positions();
    let infos = result.get_glyph_infos();

    let mut cursor = (10*64, 100*64);
    let mut vec = Vec::with_capacity(result.len());
    // iterate over the shaped glyphs
    for (position, info) in positions.iter().zip(infos) {
        let gid = info.codepoint;
        let x_advance = position.x_advance;
        let x_offset = position.x_offset;
        let y_offset = position.y_offset;

        let x_pos: f64 = (cursor.0 + x_offset) as f64 / 64.0;
        let y_pos: f64 = (cursor.1 + y_offset) as f64 / 64.0;

        // separate x_pos and y_pos into integral an fractional parts for subpixel positioning.
        let x_pos_int = x_pos.trunc();
        let x_pos_frac = x_pos.fract();
        let y_pos_int = y_pos.trunc();
        let y_pos_frac = y_pos.fract();

        // no metrics hinting
        cursor.0 += x_advance;

        let glyph = match rt_font.font.glyph(rusttype::GlyphId(gid)) {
            Some(glyph) => glyph,
            None => continue,
        };

        let glyph = glyph.scaled(rt_font.scale).positioned(rusttype::Point {
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

        let texture = Texture::from_image(&mut win.factory, &image_buffer, &TextureSettings::new())
            .unwrap();

        vec.push((image, texture));
    }

    vec
}

fn draw_glyph(glyph: &rusttype::PositionedGlyph) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let bbox = glyph.pixel_bounding_box().unwrap();
    let mut image_buffer = ImageBuffer::new((bbox.max.x - bbox.min.x) as u32,
                                            (bbox.max.y - bbox.min.y) as u32);
    glyph.draw(|x, y, v| {
        let v = (255.0 * v).round() as u8;
        image_buffer.put_pixel(x, y, Rgba { data: [0, 0, 0, v] })
    });
    image_buffer
}
