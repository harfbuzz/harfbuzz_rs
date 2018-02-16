use hb;
use std;

use libc::c_void;

pub use font_funcs::FontFuncs;
use font_funcs::FontFuncsImpl;
use face::Face;
use common::{HarfbuzzObject, Owned, Shared};

use std::marker::PhantomData;
use std::ffi::CStr;

pub type Glyph = u32;
pub type FontExtents = hb::hb_font_extents_t;
pub type GlyphExtents = hb::hb_glyph_extents_t;
pub type Position = hb::hb_position_t;

pub(crate) extern "C" fn destroy_box<U>(ptr: *mut c_void) {
    unsafe { Box::from_raw(ptr as *mut U) };
}

/// A font is the most important concept in harfbuzz.
///
/// A font can be created either as a subfont of an existing font or directly from a `Face` using
/// its `create_font` function.
#[derive(Debug)]
#[repr(C)]
pub struct Font<'a> {
    _raw: hb::hb_font_t,
    _marker: PhantomData<&'a hb::hb_font_t>,
}

impl<'a> Font<'a> {
    /// Create a new font with a specified `Face`.
    pub fn new<T: Into<Shared<Face<'a>>>>(face: T) -> Owned<Self> {
        unsafe {
            let face = face.into();
            let raw_font = hb::hb_font_create(Shared::into_raw(face));
            // set default font funcs for a completely new font
            // hb::hb_ot_font_set_funcs(raw_font);
            Owned::from_raw(raw_font)
        }
    }

    /// Create a new sub font from the current font that by default inherits its parent font's
    /// face, scale, ppem and font funcs.
    pub fn create_sub_font<T: Into<Shared<Font<'a>>>>(font: T) -> Owned<Font<'a>> {
        unsafe { Owned::from_raw(hb::hb_font_create_sub_font(font.into().as_raw())) }
    }

    /// Returns the parent font.
    pub fn parent(&self) -> &Font<'a> {
        unsafe { Font::from_raw(hb::hb_font_get_parent(self.as_raw())) }
    }

    /// Returns the face which was used to create the font.
    pub fn face(&self) -> &Face<'a> {
        unsafe { Face::from_raw(hb::hb_font_get_face(self.as_raw())) }
    }

    pub fn scale(&self) -> (i32, i32) {
        let mut result = (0i32, 0i32);
        unsafe { hb::hb_font_get_scale(self.as_raw(), &mut result.0, &mut result.1) };
        result
    }

    pub fn set_scale(&mut self, x: i32, y: i32) -> &mut Font<'a> {
        unsafe { hb::hb_font_set_scale(self.as_raw(), x, y) };
        self
    }

    pub fn ppem(&self) -> (u32, u32) {
        let mut result = (0u32, 0u32);
        unsafe { hb::hb_font_get_ppem(self.as_raw(), &mut result.0, &mut result.1) };
        result
    }

    pub fn set_ppem(&mut self, x: u32, y: u32) -> &mut Font<'a> {
        unsafe { hb::hb_font_set_ppem(self.as_raw(), x, y) };
        self
    }

    pub fn set_font_funcs<FuncsType>(&mut self, funcs: FuncsType) -> &mut Font<'a>
    where
        FuncsType: 'a + Send + FontFuncs,
    {
        let funcs_impl: Owned<FontFuncsImpl<FuncsType>> = FontFuncsImpl::from_trait_impl();
        let font_data = Box::new(funcs);
        unsafe {
            hb::hb_font_set_funcs(
                self.as_raw(),
                funcs_impl.as_raw(),
                Box::into_raw(font_data) as *mut _,
                Some(destroy_box::<FuncsType>),
            )
        };
        self
    }

    // scale from parent font
    pub(crate) fn parent_scale_x_distance(&self, v: Position) -> Position {
        let parent_x_scale = self.parent().scale().0;
        let x_scale = self.scale().0;
        if parent_x_scale != x_scale {
            (v as i64 * x_scale as i64 / parent_x_scale as i64) as Position
        } else {
            v
        }
    }

    // scale from parent font
    pub(crate) fn parent_scale_y_distance(&self, v: Position) -> Position {
        let parent_y_scale = self.parent().scale().1;
        let y_scale = self.scale().1;
        if parent_y_scale != y_scale {
            (v as i64 * y_scale as i64 / parent_y_scale as i64) as Position
        } else {
            v
        }
    }

    // scale from parent font
    pub(crate) fn parent_scale_position(&self, v: (Position, Position)) -> (Position, Position) {
        (
            self.parent_scale_x_distance(v.0),
            self.parent_scale_y_distance(v.1),
        )
    }

    pub fn get_font_h_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<FontExtents>();
            let result = hb::hb_font_get_h_extents(self.as_raw(), &mut extents);
            if result == 1 {
                Some(extents)
            } else {
                None
            }
        }
    }

    pub fn get_font_v_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<FontExtents>();
            let result = hb::hb_font_get_v_extents(self.as_raw(), &mut extents);
            if result == 1 {
                Some(extents)
            } else {
                None
            }
        }
    }

    pub fn get_nominal_glyph(&self, c: char) -> Option<Glyph> {
        unsafe {
            let mut glyph = 0;
            let result = hb::hb_font_get_nominal_glyph(self.as_raw(), c as u32, &mut glyph);
            if result == 1 {
                Some(glyph)
            } else {
                None
            }
        }
    }

    pub fn get_variation_glyph(&self, c: char, v: char) -> Option<Glyph> {
        unsafe {
            let mut glyph = 0;
            let result =
                hb::hb_font_get_variation_glyph(self.as_raw(), c as u32, v as u32, &mut glyph);
            if result == 1 {
                Some(glyph)
            } else {
                None
            }
        }
    }

    pub fn get_glyph_h_advance(&self, glyph: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_h_advance(self.as_raw(), glyph) }
    }

    pub fn get_glyph_v_advance(&self, glyph: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_v_advance(self.as_raw(), glyph) }
    }

    pub fn get_glyph_h_origin(&self, glyph: Glyph) -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result =
                hb::hb_font_get_glyph_h_origin(self.as_raw(), glyph, &mut pos.0, &mut pos.1);
            if result == 1 {
                Some(pos)
            } else {
                None
            }
        }
    }

    pub fn get_glyph_v_origin(&self, glyph: Glyph) -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result =
                hb::hb_font_get_glyph_v_origin(self.as_raw(), glyph, &mut pos.0, &mut pos.1);
            if result == 1 {
                Some(pos)
            } else {
                None
            }
        }
    }

    pub fn get_glyph_h_kerning(&self, left: Glyph, right: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_h_kerning(self.as_raw(), left, right) }
    }

    pub fn get_glyph_v_kerning(&self, before: Glyph, after: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_v_kerning(self.as_raw(), before, after) }
    }

    pub fn get_glyph_extents(&self, glyph: Glyph) -> Option<GlyphExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<GlyphExtents>();
            let result = hb::hb_font_get_glyph_extents(self.as_raw(), glyph, &mut extents);
            if result == 1 {
                Some(extents)
            } else {
                None
            }
        }
    }

    pub fn get_glyph_contour_point(
        &self,
        glyph: Glyph,
        point_index: u32,
    ) -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result = hb::hb_font_get_glyph_contour_point(
                self.as_raw(),
                glyph,
                point_index,
                &mut pos.0,
                &mut pos.1,
            );
            if result == 1 {
                Some(pos)
            } else {
                None
            }
        }
    }

    pub fn get_glyph_name(&self, glyph: Glyph) -> Option<String> {
        let mut buffer = [0; 256];
        let result = unsafe {
            hb::hb_font_get_glyph_name(
                self.as_raw(),
                glyph,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
            )
        };
        if result == 1 {
            let cstr = unsafe { CStr::from_ptr(buffer.as_ptr()) };
            cstr.to_str().ok().map(|y| y.to_string())
        } else {
            None
        }
    }

    pub fn get_glyph_from_name(&self, name: &str) -> Option<Glyph> {
        unsafe {
            let mut glyph = 0;
            let result = hb::hb_font_get_glyph_from_name(
                self.as_raw(),
                name.as_ptr() as *mut _,
                name.len() as i32,
                &mut glyph,
            );
            if result == 1 {
                Some(glyph)
            } else {
                None
            }
        }
    }
}

impl<'a> HarfbuzzObject for Font<'a> {
    type Raw = hb::hb_font_t;

    unsafe fn reference(&self) {
        hb::hb_font_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb::hb_font_destroy(self.as_raw());
    }
}
