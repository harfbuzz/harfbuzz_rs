use hb;
use std;
use face::Face;

use std::marker::PhantomData;

pub type Glyph = u32;
pub type FontExtents = hb::hb_font_extents_t;
pub type GlyphExtents = hb::hb_glyph_extents_t;
pub type Position = hb::hb_position_t;


/// An instance of a font at a specific scale.
#[derive(Debug)]
pub struct Font<'a> {
    hb_font: *mut hb::hb_font_t,
    _marker: PhantomData<&'a hb::hb_font_t>,
}

impl<'a> Font<'a> {
    pub unsafe fn from_raw(font: *mut hb::hb_font_t) -> Font<'a> {
        Font {
            hb_font: font,
            _marker: PhantomData,
        }
    }

    pub fn face(&self) -> Face<'a> {
        unsafe {
            let raw_face = hb::hb_font_get_face(self.hb_font);
            hb::hb_face_reference(raw_face);
            Face::from_raw(raw_face)
        }
    }

    pub fn as_raw(&self) -> *mut hb::hb_font_t {
        self.hb_font
    }

    pub fn get_glyph_h_advance(&self, glyph: Glyph) -> i32 {
        unsafe { hb::hb_font_get_glyph_h_advance(self.hb_font, glyph) }
    }

    pub fn get_glyph_v_advance(&self, glyph: Glyph) -> i32 {
        unsafe { hb::hb_font_get_glyph_v_advance(self.hb_font, glyph) }
    }

    pub fn get_font_h_extents(&self) -> FontExtents {
        unsafe {
            let mut extents = std::mem::uninitialized::<FontExtents>();
            hb::hb_font_get_h_extents(self.hb_font, &mut extents as *mut FontExtents);
            extents
        }
    }

    pub fn scale(&self) -> (i32, i32) {
        let mut result = (0i32, 0i32);
        unsafe { hb::hb_font_get_scale(self.hb_font, &mut result.0, &mut result.1) };
        result
    }

    pub fn set_scale(&mut self, x: i32, y: i32) -> &mut Font<'a> {
        unsafe { hb::hb_font_set_scale(self.hb_font, x, y) };
        self
    }

    pub fn ppem(&self) -> (u32, u32) {
        let mut result = (0u32, 0u32);
        unsafe { hb::hb_font_get_ppem(self.hb_font, &mut result.0, &mut result.1) };
        result
    }

    pub fn set_ppem(&mut self, x: u32, y: u32) -> &mut Font<'a> {
        unsafe { hb::hb_font_set_ppem(self.hb_font, x, y) };
        self
    }

    pub fn set_font_funcs<T: FontFuncs>(&mut self, funcs: T) -> &mut Font<'a> {
        // TODO: Cache this somehow
        let raw_funcs = HbFontFuncs::new::<T>();
        let data_ptr = Box::into_raw(Box::new(funcs));

        extern "C" fn destroy_box<U>(ptr: *mut std::os::raw::c_void) {
            unsafe { Box::from_raw(ptr as *mut U) };
        }

        unsafe {
            hb::hb_font_set_funcs(self.hb_font,
                                  raw_funcs.hb_funcs,
                                  data_ptr as *mut _,
                                  Some(destroy_box::<T>));
        }
        self
    }
}

impl<'a> Clone for Font<'a> {
    fn clone(&self) -> Self {
        let hb_font = unsafe { hb::hb_font_reference(self.hb_font) };
        Font {
            hb_font: hb_font,
            _marker: PhantomData,
        }
    }
}

impl<'a> Drop for Font<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_font_destroy(self.hb_font);
        }
    }
}

pub fn font_set_harfbuzz_opentype_funcs(font: &mut Font) {
    unsafe { hb::hb_ot_font_set_funcs(font.as_raw()) }
}

/// This Trait specifies the font callbacks that harfbuzz uses. No function in this trait must
/// be implemented, however the default implementations simply return null values.
#[allow(unused_variables)]
pub trait FontFuncs {
    fn get_font_h_extents(&self, font: &Font) -> Option<FontExtents> {
        None
    }
    fn get_font_v_extents(&self, font: &Font) -> Option<FontExtents> {
        None
    }
    fn get_nominal_glyph(&self, font: &Font, unicode: char) -> Option<Glyph> {
        None
    }
    fn get_variation_glyph(&self, font: &Font, unicode: char, variation_sel: char) -> Option<Glyph> {
        None
    }
    fn get_h_advance(&self, font: &Font, glyph: Glyph) -> Position {
        0
    }
    fn get_v_advance(&self, font: &Font, glyph: Glyph) -> Position {
        0
    }
    fn get_h_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        None
    }
    fn get_v_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        None
    }
    fn get_h_kerning(&self, font: &Font, left: Glyph, right: Glyph) -> Position {
        0
    }
    fn get_v_kerning(&self, font: &Font, left: Glyph, right: Glyph) -> Position {
        0
    }
    fn get_glyph_extents(&self, font: &Font, glyph: Glyph) -> Option<GlyphExtents> {
        None
    }
    fn get_glyph_contour_point(&self, font: &Font, glyph: Glyph, point_index: u32) -> Option<(u32, u32)> {
        None
    }
    fn get_glyph_name(&self, font: &Font, glyph: Glyph) -> Option<String> {
        None
    }
    fn get_glyph_from_name(&self, font: &Font, name: &str) -> Option<Glyph> {
        None
    }
}


extern "C" fn rust_get_font_h_extents<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                    font_data: *mut std::os::raw::c_void,
                                                    metrics: *mut FontExtents,
                                                    _: *mut std::os::raw::c_void)
                                                    -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    let result = funcs.get_font_h_extents(font);

    if let Some(extents) = result {
        unsafe { *metrics = extents };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_font_v_extents<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                    font_data: *mut std::os::raw::c_void,
                                                    metrics: *mut FontExtents,
                                                    _: *mut std::os::raw::c_void)
                                                    -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    let result = funcs.get_font_v_extents(font);

    if let Some(extents) = result {
        unsafe { *metrics = extents };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_nominal_glyph<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   unicode: hb::hb_codepoint_t,
                                                   glyph: *mut hb::hb_codepoint_t,
                                                   _: *mut std::os::raw::c_void)
                                                   -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    let unicode = std::char::from_u32(unicode);
    match unicode {
        Some(unicode) => {
            let result = funcs.get_nominal_glyph(font, unicode);
            if let Some(result_glyph) = result {
                unsafe { *glyph = result_glyph };
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

extern "C" fn rust_get_variation_glyph<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     unicode: hb::hb_codepoint_t,
                                                     variation_selector: hb::hb_codepoint_t,
                                                     glyph: *mut hb::hb_codepoint_t,
                                                     _: *mut std::os::raw::c_void)
                                                     -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    let unicode = std::char::from_u32(unicode);
    let variation_selector = std::char::from_u32(variation_selector);
    match (unicode, variation_selector) {
        (Some(unicode), Some(variation_selector)) => {
            let result = funcs.get_variation_glyph(font, unicode, variation_selector);
            if let Some(result_glyph) = result {
                unsafe { *glyph = result_glyph };
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

extern "C" fn rust_get_glyph_h_advance<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     glyph: hb::hb_codepoint_t,
                                                     _: *mut std::os::raw::c_void)
                                                     -> Position {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    funcs.get_h_advance(font, glyph)
}

extern "C" fn rust_get_glyph_v_advance<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     glyph: hb::hb_codepoint_t,
                                                     _: *mut std::os::raw::c_void)
                                                     -> Position {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    funcs.get_v_advance(font, glyph)
}

extern "C" fn rust_get_glyph_h_origin<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                    font_data: *mut std::os::raw::c_void,
                                                    glyph: hb::hb_codepoint_t,
                                                    x: *mut Position,
                                                    y: *mut Position,
                                                    _: *mut std::os::raw::c_void)
                                                    -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    if let Some((x_origin, y_origin)) = funcs.get_h_origin(font, glyph) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_v_origin<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                    font_data: *mut std::os::raw::c_void,
                                                    glyph: hb::hb_codepoint_t,
                                                    x: *mut Position,
                                                    y: *mut Position,
                                                    _: *mut std::os::raw::c_void)
                                                    -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    if let Some((x_origin, y_origin)) = funcs.get_v_origin(font, glyph) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_h_kerning<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     left: hb::hb_codepoint_t,
                                                     right: hb::hb_codepoint_t,
                                                     _: *mut std::os::raw::c_void)
                                                     -> Position {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    funcs.get_h_kerning(font, left, right)
}

extern "C" fn rust_get_glyph_v_kerning<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     left: hb::hb_codepoint_t,
                                                     right: hb::hb_codepoint_t,
                                                     _: *mut std::os::raw::c_void)
                                                     -> Position {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    funcs.get_v_kerning(font, left, right)
}

extern "C" fn rust_get_glyph_extents<T: FontFuncs>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   glyph: hb::hb_codepoint_t,
                                                   extents: *mut hb::hb_glyph_extents_t,
                                                   _: *mut std::os::raw::c_void)
                                                   -> hb::hb_bool_t {
    let funcs = unsafe { &*(font_data as *const T) };
    let font = unsafe { &*(font as *const Font) };
    match funcs.get_glyph_extents(font, glyph) {
        Some(result) => {
            unsafe { *extents = result };
            1
        }
        None => 0,
    }
}

macro_rules! create_font_funcs {
    ($x:path) => (HbFontFuncs::new::<$x>());
}

pub struct HbFontFuncs {
    hb_funcs: *mut hb::hb_font_funcs_t,
}

#[allow(new_without_default)]
impl HbFontFuncs {
    pub fn new<T: FontFuncs>() -> HbFontFuncs {
        unsafe {
            let hb_funcs = hb::hb_font_funcs_create();
            hb::hb_font_funcs_set_font_h_extents_func(hb_funcs,
                                                      Some(rust_get_font_h_extents::<T>),
                                                      std::ptr::null_mut(),
                                                      None);
            hb::hb_font_funcs_set_font_v_extents_func(hb_funcs,
                                                      Some(rust_get_font_v_extents::<T>),
                                                      std::ptr::null_mut(),
                                                      None);
            hb::hb_font_funcs_set_nominal_glyph_func(hb_funcs,
                                                     Some(rust_get_nominal_glyph::<T>),
                                                     std::ptr::null_mut(),
                                                     None);
            hb::hb_font_funcs_set_variation_glyph_func(hb_funcs,
                                                       Some(rust_get_variation_glyph::<T>),
                                                       std::ptr::null_mut(),
                                                       None);
            hb::hb_font_funcs_set_glyph_h_advance_func(hb_funcs,
                                                       Some(rust_get_glyph_h_advance::<T>),
                                                       std::ptr::null_mut(),
                                                       None);
            hb::hb_font_funcs_set_glyph_v_advance_func(hb_funcs,
                                                       Some(rust_get_glyph_v_advance::<T>),
                                                       std::ptr::null_mut(),
                                                       None);
            hb::hb_font_funcs_set_glyph_h_origin_func(hb_funcs,
                                                      Some(rust_get_glyph_h_origin::<T>),
                                                      std::ptr::null_mut(),
                                                      None);
            hb::hb_font_funcs_set_glyph_v_origin_func(hb_funcs,
                                                      Some(rust_get_glyph_v_origin::<T>),
                                                      std::ptr::null_mut(),
                                                      None);
            hb::hb_font_funcs_set_glyph_h_kerning_func(hb_funcs,
                                                       Some(rust_get_glyph_h_kerning::<T>),
                                                       std::ptr::null_mut(),
                                                       None);
            hb::hb_font_funcs_set_glyph_v_kerning_func(hb_funcs,
                                                       Some(rust_get_glyph_v_kerning::<T>),
                                                       std::ptr::null_mut(),
                                                       None);
            hb::hb_font_funcs_set_glyph_extents_func(hb_funcs,
                                                     Some(rust_get_glyph_extents::<T>),
                                                     std::ptr::null_mut(),
                                                     None);
            hb::hb_font_funcs_make_immutable(hb_funcs);
            HbFontFuncs { hb_funcs: hb_funcs }
        }
    }
}



#[cfg(test)]
mod tests {
    use std::default::Default;
    use super::*;
    use face::Face;

    struct MyFontFuncs {}
    impl FontFuncs for MyFontFuncs {
        fn get_font_h_extents(&self, _: &Font) -> Option<FontExtents> {
            let extents = FontExtents { ascender: 1212, ..Default::default() };
            Some(extents)
        }
    }

    #[test]
    fn font_func_test() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let mut font = Face::new(&font_bytes[..], 0).create_font();

        let my_funcs = create_font_funcs!(MyFontFuncs);
        //font.set_font_funcs(my_funcs);

        println!("{:?}", font.get_font_h_extents());
        assert_eq!(1212, font.get_font_h_extents().ascender);
    }
}
