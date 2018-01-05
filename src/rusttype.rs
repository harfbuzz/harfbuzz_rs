//! This module allows you to use rusttype to provide the font operations that harfbuzz needs.

extern crate rusttype;

use common::Tag;
use self::rusttype::{GlyphId, Scale, Codepoint};
use self::rusttype::Font as RTFont;

use font;
use face;
use font::{FontFuncs, Glyph as GlyphIndex, Position, FontFuncsImpl, Font as HBFont, GlyphExtents};
use common::{HbArc, HbRef};

use std::str::FromStr;

lazy_static! {
    /// This is a global static that derefs into a `FontFuncsImpl` and can be used when you want
    /// rusttype to provide the font funcs for harfbuzz.
    pub static ref RT_FONT_FUNCS: HbArc<FontFuncsImpl<ScaledRusttypeFont<'static>>> = {
        FontFuncsImpl::from_trait_impl().into()
    };
}

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

pub fn rusttype_font_from_face<'a>(face: &face::Face<'a>) -> RTFont<'a> {
    let font_blob = face.face_data();
    let index = face.index();
    let collection = rusttype::FontCollection::from_bytes(font_blob);
    collection.font_at(index as usize).unwrap()
}

fn rusttype_scale_from_hb_font(font: &font::Font) -> Scale {
    let font_height = get_font_height(font) as f32;
    let em_scale = font.scale();
    let x_scale = em_scale.0 as f32;
    let y_scale = em_scale.1 as f32;
    Scale {
        x: font_height * x_scale / y_scale,
        y: font_height,
    }
}

pub struct ScaledRusttypeFont<'a> {
    pub font: rusttype::Font<'a>,
    pub scale: Scale,
}

impl<'a> ScaledRusttypeFont<'a> {
    pub fn from_hb_font(hb_font: &font::Font<'a>) -> ScaledRusttypeFont<'a> {
        let font = rusttype_font_from_face(&hb_font.face());
        let scale = rusttype_scale_from_hb_font(hb_font);
        ScaledRusttypeFont {
            font: font,
            scale: scale,
        }
    }
}

impl<'a> FontFuncs for ScaledRusttypeFont<'a> {
    fn get_glyph_h_advance(&self, _: HbRef<HBFont>, glyph: GlyphIndex) -> Position {
        let glyph = self.font.glyph(GlyphId(glyph));
        if let Some(glyph) = glyph {
            let glyph = glyph.scaled(self.scale);
            glyph.h_metrics().advance_width.round() as Position
        } else {
            0
        }
    }

    fn get_glyph_h_kerning(&self, _: HbRef<HBFont>, left: GlyphIndex, right: GlyphIndex) -> Position {
        self.font.pair_kerning(self.scale, GlyphId(left), GlyphId(right)).round() as Position
    }

    fn get_glyph_extents(&self, _: HbRef<HBFont>, glyph: GlyphIndex) -> Option<GlyphExtents> {
        let glyph = self.font.glyph(GlyphId(glyph));
        glyph.and_then(|glyph| {
            let glyph = glyph.scaled(self.scale);
            glyph.exact_bounding_box().map(|bbox| {
                                               GlyphExtents {
                                                   x_bearing: bbox.min.x.round() as i32,
                                                   y_bearing: bbox.min.y.round() as i32,
                                                   width: (bbox.max.x - bbox.min.x).round() as i32,
                                                   height: (bbox.max.y - bbox.min.y).round() as i32,
                                               }
                                           })
        })
    }

    fn get_nominal_glyph(&self, _: HbRef<font::Font>, unicode: char) -> Option<GlyphIndex> {
        self.font.glyph(Codepoint(unicode as u32)).map(|glyph| glyph.id().0)
    }
}

/// Let a font use rusttype's font API for getting information like the advance width of some
/// glyph or its extents.
pub fn font_set_rusttype_funcs(font: &mut font::Font) {
    let font_data = ScaledRusttypeFont::from_hb_font(font);
    font.set_font_funcs(RT_FONT_FUNCS.clone(), font_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use font::Font;
    use face::Face;
}
