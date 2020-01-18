//! This module allows you to use rusttype to provide the font operations that harfbuzz needs.

use crate::common::Tag;
pub use rusttype::Error;
use rusttype::Font as RTFont;
use rusttype::{Codepoint, GlyphId, Scale};

use crate::face;
use crate::font;
use crate::font::{Font, FontFuncs, Glyph as GlyphIndex, GlyphExtents, Position};

use std;
use std::fmt::Debug;
use std::str::FromStr;

// Work around weird rusttype scaling by reading the hhea table.
fn get_font_height(font: &font::Font<'_>) -> Result<i32, Error> {
    let face = font.face();
    let tag = Tag::from_str("hhea").unwrap();
    let hhea_table = face.table_with_tag(tag).ok_or(Error::IllFormed)?;
    if hhea_table.len() >= 8 {
        unsafe {
            let ascent_ptr = (&hhea_table)[4..6].as_ptr() as *const i16;
            let ascent = i16::from_be(*ascent_ptr);
            let descent_ptr = (&hhea_table)[6..8].as_ptr() as *const i16;
            let descent = i16::from_be(*descent_ptr);
            Ok(ascent as i32 - descent as i32)
        }
    } else {
        Err(Error::IllFormed)
    }
}

fn rusttype_font_from_face<'a>(face: &face::Face<'a>) -> Result<RTFont<'a>, Error> {
    // It is unfortunate that we have to copy the face data here.
    let font_blob = face.face_data().as_ref().to_owned();
    let index = face.index();
    let collection = rusttype::FontCollection::from_bytes(font_blob)?;
    collection.font_at(index as usize)
}

fn rusttype_scale_from_hb_font(font: &font::Font<'_>) -> Result<Scale, Error> {
    let font_height = get_font_height(font)? as f32;
    let em_scale = font.scale();
    let x_scale = em_scale.0 as f32;
    let y_scale = em_scale.1 as f32;
    Ok(Scale {
        x: font_height * x_scale / y_scale,
        y: font_height,
    })
}

struct ScaledRusttypeFont<'a> {
    font: rusttype::Font<'a>,
    scale: Scale,
}

impl<'a> Debug for ScaledRusttypeFont<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaledRusttypeFont")
            .field("scale", &self.scale)
            .finish()
    }
}

impl<'a> ScaledRusttypeFont<'a> {
    fn from_hb_font<'b>(hb_font: &font::Font<'b>) -> Result<ScaledRusttypeFont<'b>, Error> {
        let font = rusttype_font_from_face(&hb_font.face())?;
        let scale = rusttype_scale_from_hb_font(hb_font)?;
        Ok(ScaledRusttypeFont { font, scale })
    }
}

impl<'a> FontFuncs for ScaledRusttypeFont<'a> {
    fn get_glyph_h_advance(&self, _: &Font<'_>, glyph: GlyphIndex) -> Position {
        let glyph = self.font.glyph(GlyphId(glyph));
        let glyph = glyph.scaled(self.scale);
        glyph.h_metrics().advance_width.round() as Position
    }
    fn get_glyph_extents(&self, _: &Font<'_>, glyph: GlyphIndex) -> Option<GlyphExtents> {
        let glyph = self.font.glyph(GlyphId(glyph));
        let glyph = glyph.scaled(self.scale);
        glyph.exact_bounding_box().map(|bbox| GlyphExtents {
            x_bearing: bbox.min.x.round() as i32,
            y_bearing: bbox.min.y.round() as i32,
            width: (bbox.max.x - bbox.min.x).round() as i32,
            height: (bbox.max.y - bbox.min.y).round() as i32,
        })
    }
    fn get_nominal_glyph(&self, _: &font::Font<'_>, unicode: char) -> Option<GlyphIndex> {
        let glyph = self.font.glyph(Codepoint(unicode as u32));
        Some(glyph.id().0)
    }
}

use std::sync::Arc;

/// Creates a new HarfBuzz `Font` object that uses RustType to provide font data.
///
/// # Examples
///
/// Create a basic font that uses rusttype font funcs:
/// ```
/// use std::fs;
/// use std::sync::Arc;
///
/// use harfbuzz_rs::rusttype::create_harfbuzz_rusttype_font;
///
/// let path = "testfiles/SourceSansVariable-Roman.ttf";
/// let bytes: Arc<[u8]> = fs::read(path).unwrap().into();
/// let font = create_harfbuzz_rusttype_font(bytes, 0);
/// ```
pub fn create_harfbuzz_rusttype_font(
    bytes: impl Into<Arc<[u8]>>,
    index: u32,
) -> Result<crate::Owned<Font<'static>>, Error> {
    let bytes = bytes.into();
    let face = crate::Face::new(bytes.clone(), index);
    let mut font = Font::new(face);

    let rt_font = rusttype::FontCollection::from_bytes(bytes)?.font_at(index as usize)?;
    let scaled_font = ScaledRusttypeFont {
        font: rt_font,
        scale: rusttype_scale_from_hb_font(&font)?,
    };
    font.set_font_funcs(scaled_font);

    Ok(font)
}

/// Extends the harfbuzz font to allow setting RustType as font funcs provider.
#[deprecated(since = "0.4.0")]
pub trait SetRustTypeFuncs {
    /// Let a font use rusttype's font API for getting information like the
    /// advance width of some glyph or its extents.
    ///
    /// # Deprecated
    ///
    /// This function is deprecated because it doesn't fit well with the design
    /// of RustType (Calling this method requires to make a copy of the font
    /// data used). You should use `create_harfbuzz_rusttype_font` instead.
    #[deprecated(since = "0.4.0")]
    fn set_rusttype_funcs(&mut self) -> Result<(), Error>;
}

#[allow(deprecated)]
impl<'a> SetRustTypeFuncs for Font<'a> {
    fn set_rusttype_funcs(&mut self) -> Result<(), Error> {
        let font_data = ScaledRusttypeFont::from_hb_font(self)?;
        self.set_font_funcs(font_data);
        Ok(())
    }
}
