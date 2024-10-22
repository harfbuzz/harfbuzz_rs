//! This module allows you to create a harfbuzz font from freetype faces

use crate::Font;
use crate::bindings::hb_font_t;
use crate::common::Owned;

use freetype as ft;

extern "C" {
    fn hb_ft_font_create_referenced(face: freetype_sys::FT_Face) -> *mut hb_font_t;
}

impl<'a> Font<'a> {
    /// Create a font from a freetype face.
    ///
    /// ```
    /// let font_data = include_bytes!("../Inter-Regular.otf") as &[u8];
    /// let face = harfbuzz_rs::Face::from_bytes(font_data, 0);
    ///
    /// let ftlib = ft::Library::init().unwrap();
    /// let ft_face = ftlib.new_memory_face2(font_data, 0).unwrap();
    ///
    /// let hb_font = harfbuzz_rs::Font::from_freetype_face(ft_face.clone());
    /// ```
    pub fn from_freetype_face(mut ft_face: ft::Face<&'a [u8]>) -> Owned<Font<'a>> {
        let hb_face = unsafe {
            let ft_face_ptr: freetype_sys::FT_Face = ft_face.raw_mut();
            hb_ft_font_create_referenced(ft_face_ptr)
        };
        unsafe { Owned::from_raw(hb_face) }
    }
}

