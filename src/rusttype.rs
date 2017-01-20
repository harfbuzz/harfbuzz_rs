//! This module allows you to use rusttype to provide the font operations that harfbuzz needs.

extern crate test;
extern crate rusttype;

lazy_static! {
    static ref RT_FONT_FUNCS: FontFuncsImpl<RustTypeFontFuncs<'static>> = {
        FontFuncsImpl::from_trait_impl()
    };
}

use std::str::FromStr;

use self::rusttype::{GlyphId, Scale, Codepoint};

use font;
use font::{FontFuncs, Glyph as GlyphIndex, Position, FontFuncsImpl};
use face::FontTableAccess;
use common::Tag;

// Work around weird rusttype scaling by reading the hhea table.
fn get_font_height(font: &font::Font) -> i32 {
    let face = font.face();
    let hhea_table = face.table_with_tag(Tag::from_str("hhea").unwrap()).unwrap();
    if hhea_table.len() >= 8 {
        unsafe {
            let ascent_ptr = (&hhea_table)[4..6].as_ptr() as *const i16;
            let ascent = i16::from_be(*ascent_ptr);
            let descent_ptr = (&hhea_table)[6..8].as_ptr() as *const i16;
            let descent = i16::from_be(*descent_ptr);
            ascent as i32 - descent as i32
        }
    } else {
        0
    }
}

pub struct RustTypeFontFuncs<'a> {
    font: rusttype::Font<'a>,
    font_height: f32,
}

impl<'a> RustTypeFontFuncs<'a> {
    pub fn new_from_font<'b>(hb_font: &font::Font<'b>) -> RustTypeFontFuncs<'b> {
        let font_blob = hb_font.face().reference_blob();
        let index = hb_font.face().index();
        let collection = rusttype::FontCollection::from_bytes(font_blob.get_data());
        let font = collection.font_at(index as usize).unwrap();
        let font_height = get_font_height(hb_font);
        RustTypeFontFuncs {
            font: font,
            font_height: font_height as f32,
        }
    }
}

impl<'a> FontFuncs for RustTypeFontFuncs<'a> {
    fn get_h_advance(&self, font: &font::Font, glyph: GlyphIndex) -> Position {
        let glyph = self.font.glyph(GlyphId(glyph));
        if let Some(glyph) = glyph {
            let (scale_x, scale_y) = font.scale();
            let scale_x = scale_x as f32 * self.font_height as f32 / font.face().upem() as f32;
            let scale_y = scale_y as f32 * self.font_height as f32 / font.face().upem() as f32;
            let glyph = glyph.scaled(Scale {
                x: scale_x,
                y: scale_y,
            });
            glyph.h_metrics().advance_width.round() as i32
        } else {
            0
        }
    }

    fn get_nominal_glyph(&self, _: &font::Font, unicode: char) -> Option<GlyphIndex> {
        self.font.glyph(Codepoint(unicode as u32)).map(|glyph| glyph.id().0)
    }
}

/// Let a font use rusttype's font API for getting information like the advance width of some
/// glyph or its extents.
pub fn font_set_rusttype_funcs(font: &mut font::Font) {
    let font_data = RustTypeFontFuncs::new_from_font(font);
    font.set_font_funcs(&RT_FONT_FUNCS, font_data);
}

pub fn font_set_rusttype_funcs2(font: &mut font::Font) {
    let font_data = RustTypeFontFuncs::new_from_font(font);
    let ffuncs = FontFuncsImpl::from_trait_impl();
    font.set_font_funcs(&ffuncs, font_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use face::Face;

    #[test]
    fn test_basic_rusttype() {
        let font_bytes = include_bytes!("../testfiles/Optima.ttc");
        let face = Face::new(&font_bytes[..], 0);
        let upem = face.upem();
        println!("upem: {:?}", upem);
        let mut font = face.create_font();

        font.set_scale(100, 100);

        let before = font.get_glyph_h_advance(47);
        font_set_rusttype_funcs(&mut font);
        let after = font.get_glyph_h_advance(47);
        println!("{:?} == {:?}", before, after);
        assert_eq!(before, after);
    }

    use self::test::Bencher;
    #[bench]
    fn bench_rusttype(b: &mut Bencher) {
        let font_bytes = include_bytes!("../testfiles/Optima.ttc");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = face.create_font();
        font.set_scale(100, 100);
        font_set_rusttype_funcs(&mut font);
        b.iter(|| {
            for i in 1..10 {
                font.get_glyph_h_advance(i);
            }
        });
    }

    #[bench]
    fn bench_rusttype2(b: &mut Bencher) {
        let font_bytes = include_bytes!("../testfiles/Optima.ttc");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = face.create_font();
        font.set_scale(100, 100);
        font_set_rusttype_funcs2(&mut font);
        b.iter(|| {
            for i in 1..10 {
                font.get_glyph_h_advance(i);
            }
        });
    }

    #[bench]
    fn bench_rusttype3(b: &mut Bencher) {
        let font_bytes = include_bytes!("../testfiles/Optima.ttc");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = face.create_font();
        font.set_scale(100, 100);
        b.iter(|| {
            for i in 1..10 {
                font.get_glyph_h_advance(i);
            }
        });
    }
}
