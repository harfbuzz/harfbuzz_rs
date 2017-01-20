use hb;
use std;
use face::Face;
use common::HarfbuzzObject;

use std::marker::PhantomData;
use std::io::Write;
use std::ffi::{CStr, CString};

pub type Glyph = u32;
pub type FontExtents = hb::hb_font_extents_t;
pub type GlyphExtents = hb::hb_glyph_extents_t;
pub type Position = hb::hb_position_t;

/// Represents a value that is either owned or a reference.
pub enum MaybeOwned<'a, T: 'a> {
    /// Owned value.
    Owned(T),
    /// Reference to a value.
    Ref(&'a T),
}

impl<'a, T: 'a> std::convert::From<T> for MaybeOwned<'a, T> {
    fn from(val: T) -> MaybeOwned<'a, T> {
        MaybeOwned::Owned(val)
    }
}

impl<'a, T: 'a> std::convert::From<&'a T> for MaybeOwned<'a, T> {
    fn from(val: &'a T) -> MaybeOwned<'a, T> {
        MaybeOwned::Ref(val)
    }
}

impl<'a, T> std::ops::Deref for MaybeOwned<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match *self {
            MaybeOwned::Owned(ref val) => val,
            MaybeOwned::Ref(val) => val,
        }
    }
}

extern "C" fn destroy_box<U>(ptr: *mut std::os::raw::c_void) {
    unsafe { Box::from_raw(ptr as *mut U) };
}

/// A font.
///
/// A font can be created either as a subfont of an existing font or directly from a `Face` using
/// the `create_font` function.
#[derive(Debug)]
pub struct Font<'a> {
    hb_font: *mut hb::hb_font_t,
    _marker: PhantomData<&'a hb::hb_font_t>,
}

impl<'a> Font<'a> {
    pub fn create_sub_font(&self) -> Font<'a> {
        unsafe { Font::from_raw(hb::hb_font_create_sub_font(self.hb_font)) }
    }

    pub fn parent(&self) -> Font<'a> {
        unsafe { Font::from_raw_referenced(hb::hb_font_get_parent(self.hb_font)) }
    }

    pub fn face(&self) -> Face<'a> {
        unsafe {
            let raw_face = hb::hb_font_get_face(self.hb_font);
            hb::hb_face_reference(raw_face);
            Face::from_raw(raw_face)
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

    pub fn set_font_funcs<T, U>(&mut self, funcs: &FontFuncsImpl<T>, font_data: U) -> &mut Font<'a>
        where T: 'a,
              U: Into<MaybeOwned<'a, T>>
    {
        match font_data.into() {
            MaybeOwned::Owned(font_data) => unsafe {
                let font_data = Box::new(font_data);
                hb::hb_font_set_funcs(self.hb_font,
                                      funcs.raw,
                                      Box::into_raw(font_data) as *mut _,
                                      Some(destroy_box::<T>));
            },
            // TODO: this may be unsafe because we cannot ensure that font_data lives long enough
            MaybeOwned::Ref(font_data) => unsafe {
                hb::hb_font_set_funcs(self.hb_font,
                                      funcs.raw,
                                      font_data as *const T as *mut _,
                                      None);
            },
        }
        self
    }

    // scale from parent font
    fn parent_scale_x_distance(&self, v: Position) -> Position {
        let parent_x_scale = self.parent().scale().0;
        let x_scale = self.scale().0;
        if parent_x_scale != x_scale {
            (v as i64 * x_scale as i64 / parent_x_scale as i64) as Position
        } else {
            v
        }
    }

    // scale from parent font
    fn parent_scale_y_distance(&self, v: Position) -> Position {
        let parent_y_scale = self.parent().scale().1;
        let y_scale = self.scale().1;
        if parent_y_scale != y_scale {
            (v as i64 * y_scale as i64 / parent_y_scale as i64) as Position
        } else {
            v
        }
    }

    // scale from parent font
    fn parent_scale_position(&self, v: (Position, Position)) -> (Position, Position) {
        (self.parent_scale_x_distance(v.0), self.parent_scale_y_distance(v.1))
    }

    pub fn get_font_h_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<FontExtents>();
            let result = hb::hb_font_get_h_extents(self.hb_font, &mut extents);
            if result == 1 { Some(extents) } else { None }
        }
    }

    pub fn get_font_v_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<FontExtents>();
            let result = hb::hb_font_get_v_extents(self.hb_font, &mut extents);
            if result == 1 { Some(extents) } else { None }
        }
    }

    pub fn get_nominal_glyph(&self, c: char) -> Option<Glyph> {
        unsafe {
            let mut glyph = 0;
            let result = hb::hb_font_get_nominal_glyph(self.hb_font, c as u32, &mut glyph);
            if result == 1 { Some(glyph) } else { None }
        }
    }

    pub fn get_variation_glyph(&self, c: char, v: char) -> Option<Glyph> {
        unsafe {
            let mut glyph = 0;
            let result =
                hb::hb_font_get_variation_glyph(self.hb_font, c as u32, v as u32, &mut glyph);
            if result == 1 { Some(glyph) } else { None }
        }
    }

    pub fn get_glyph_h_advance(&self, glyph: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_h_advance(self.hb_font, glyph) }
    }

    pub fn get_glyph_v_advance(&self, glyph: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_v_advance(self.hb_font, glyph) }
    }

    pub fn get_glyph_h_origin(&self, glyph: Glyph) -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result =
                hb::hb_font_get_glyph_h_origin(self.hb_font, glyph, &mut pos.0, &mut pos.1);
            if result == 1 { Some(pos) } else { None }
        }
    }

    pub fn get_glyph_v_origin(&self, glyph: Glyph) -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result =
                hb::hb_font_get_glyph_v_origin(self.hb_font, glyph, &mut pos.0, &mut pos.1);
            if result == 1 { Some(pos) } else { None }
        }
    }

    pub fn get_glyph_h_kerning(&self, left: Glyph, right: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_h_kerning(self.hb_font, left, right) }
    }

    pub fn get_glyph_v_kerning(&self, before: Glyph, after: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_v_kerning(self.hb_font, before, after) }
    }

    pub fn get_glyph_extents(&self, glyph: Glyph) -> Option<GlyphExtents> {
        unsafe {
            let mut extents = std::mem::uninitialized::<GlyphExtents>();
            let result = hb::hb_font_get_glyph_extents(self.hb_font, glyph, &mut extents);
            if result == 1 { Some(extents) } else { None }
        }
    }

    pub fn get_glyph_contour_point(&self,
                                   glyph: Glyph,
                                   point_index: u32)
                                   -> Option<(Position, Position)> {
        unsafe {
            let mut pos = (0, 0);
            let result = hb::hb_font_get_glyph_contour_point(self.hb_font,
                                                             glyph,
                                                             point_index,
                                                             &mut pos.0,
                                                             &mut pos.1);
            if result == 1 { Some(pos) } else { None }
        }
    }

    pub fn get_glyph_name(&self, glyph: Glyph) -> Option<String> {
        let mut buffer = [0; 256];
        let result = unsafe {
            hb::hb_font_get_glyph_name(self.hb_font,
                                       glyph,
                                       buffer.as_mut_ptr() as *mut _,
                                       buffer.len() as u32)
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
            let result = hb::hb_font_get_glyph_from_name(self.hb_font,
                                                         name.as_ptr() as *mut _,
                                                         name.len() as i32,
                                                         &mut glyph);
            if result == 1 { Some(glyph) } else { None }
        }
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

impl<'a> HarfbuzzObject for Font<'a> {
    type Raw = *mut hb::hb_font_t;

    unsafe fn from_raw(raw: *mut hb::hb_font_t) -> Self {
        Font {
            hb_font: raw,
            _marker: PhantomData,
        }
    }

    fn as_raw(&self) -> Self::Raw {
        self.hb_font
    }
}

impl<'a> Drop for Font<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_font_destroy(self.hb_font);
        }
    }
}

/// This Trait specifies the font callbacks that harfbuzz uses. No function in this trait need to
/// be implemented, the default implementations simply return the parent font's implementation. If
/// a `Font` is created directly from a face, its parent is the empty `Font` which returns null
/// values for every font func.
#[allow(unused_variables)]
pub trait FontFuncs {
    fn get_font_h_extents(&self, font: &Font) -> Option<FontExtents> {
        font.parent().get_font_h_extents().map(|extents| {
            FontExtents {
                ascender: font.parent_scale_y_distance(extents.ascender),
                descender: font.parent_scale_y_distance(extents.descender),
                line_gap: font.parent_scale_y_distance(extents.line_gap),
                ..extents
            }
        })
    }
    fn get_font_v_extents(&self, font: &Font) -> Option<FontExtents> {
        font.parent().get_font_v_extents().map(|extents| {
            FontExtents {
                ascender: font.parent_scale_y_distance(extents.ascender),
                descender: font.parent_scale_y_distance(extents.descender),
                line_gap: font.parent_scale_y_distance(extents.line_gap),
                ..extents
            }
        })
    }
    fn get_nominal_glyph(&self, font: &Font, unicode: char) -> Option<Glyph> {
        font.parent().get_nominal_glyph(unicode)
    }
    fn get_variation_glyph(&self,
                           font: &Font,
                           unicode: char,
                           variation_sel: char)
                           -> Option<Glyph> {
        font.parent().get_variation_glyph(unicode, variation_sel)
    }
    fn get_glyph_h_advance(&self, font: &Font, glyph: Glyph) -> Position {
        font.parent_scale_x_distance(font.parent().get_glyph_h_advance(glyph))
    }
    fn get_glyph_v_advance(&self, font: &Font, glyph: Glyph) -> Position {
        font.parent_scale_y_distance(font.parent().get_glyph_v_advance(glyph))
    }
    fn get_glyph_h_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent().get_glyph_h_origin(glyph).map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_v_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent().get_glyph_v_origin(glyph).map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_h_kerning(&self, font: &Font, left: Glyph, right: Glyph) -> Position {
        font.parent_scale_x_distance(font.parent().get_glyph_h_kerning(left, right))
    }
    fn get_glyph_v_kerning(&self, font: &Font, before: Glyph, after: Glyph) -> Position {
        font.parent_scale_y_distance(font.parent().get_glyph_v_kerning(before, after))
    }
    fn get_glyph_extents(&self, font: &Font, glyph: Glyph) -> Option<GlyphExtents> {
        font.parent().get_glyph_extents(glyph).map(|extents| {
            GlyphExtents {
                x_bearing: font.parent_scale_x_distance(extents.x_bearing),
                y_bearing: font.parent_scale_y_distance(extents.y_bearing),
                width: font.parent_scale_x_distance(extents.width),
                height: font.parent_scale_y_distance(extents.height),
                ..extents
            }
        })
    }
    fn get_glyph_contour_point(&self,
                               font: &Font,
                               glyph: Glyph,
                               point_index: u32)
                               -> Option<(Position, Position)> {
        font.parent()
            .get_glyph_contour_point(glyph, point_index)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_name(&self, font: &Font, glyph: Glyph) -> Option<String> {
        font.parent().get_glyph_name(glyph)
    }
    fn get_glyph_from_name(&self, font: &Font, name: &str) -> Option<Glyph> {
        font.parent().get_glyph_from_name(name)
    }
}


extern "C" fn rust_get_font_extents_closure<T, F>(font: *mut hb::hb_font_t,
                                                  font_data: *mut std::os::raw::c_void,
                                                  metrics: *mut FontExtents,
                                                  closure_data: *mut std::os::raw::c_void)
                                                  -> hb::hb_bool_t
    where F: Fn(&Font, &T) -> Option<FontExtents>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let result = closure(font, font_data);

    if let Some(extents) = result {
        unsafe { *metrics = extents };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_nominal_glyph_closure<T, F>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   unicode: hb::hb_codepoint_t,
                                                   glyph: *mut hb::hb_codepoint_t,
                                                   closure_data: *mut std::os::raw::c_void)
                                                   -> hb::hb_bool_t
    where F: Fn(&Font, &T, char) -> Option<Glyph>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let unicode = std::char::from_u32(unicode);
    match unicode {
        Some(unicode) => {
            let result = closure(font, font_data, unicode);
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

extern "C" fn rust_get_variation_glyph_closure<T, F>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     unicode: hb::hb_codepoint_t,
                                                     variation_selector: hb::hb_codepoint_t,
                                                     glyph: *mut hb::hb_codepoint_t,
                                                     closure_data: *mut std::os::raw::c_void)
                                                     -> hb::hb_bool_t
    where F: Fn(&Font, &T, char, char) -> Option<Glyph>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let unicode = std::char::from_u32(unicode);
    let variation_selector = std::char::from_u32(variation_selector);
    match (unicode, variation_selector) {
        (Some(unicode), Some(variation_selector)) => {
            let result = closure(font, font_data, unicode, variation_selector);
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

extern "C" fn rust_get_glyph_advance_closure<T, F>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   glyph: hb::hb_codepoint_t,
                                                   closure_data: *mut std::os::raw::c_void)
                                                   -> Position
    where F: Fn(&Font, &T, Glyph) -> Position
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    closure(font, font_data, glyph)
}

extern "C" fn rust_get_glyph_origin_closure<T, F>(font: *mut hb::hb_font_t,
                                                  font_data: *mut std::os::raw::c_void,
                                                  glyph: hb::hb_codepoint_t,
                                                  x: *mut Position,
                                                  y: *mut Position,
                                                  closure_data: *mut std::os::raw::c_void)
                                                  -> hb::hb_bool_t
    where F: Fn(&Font, &T, Glyph) -> Option<(Position, Position)>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    if let Some((x_origin, y_origin)) = closure(font, font_data, glyph) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_kerning_closure<T, F>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   before: hb::hb_codepoint_t,
                                                   after: hb::hb_codepoint_t,
                                                   closure_data: *mut std::os::raw::c_void)
                                                   -> Position
    where F: Fn(&Font, &T, Glyph, Glyph) -> Position
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    closure(font, font_data, before, after)
}

extern "C" fn rust_get_glyph_extents_closure<T, F>(font: *mut hb::hb_font_t,
                                                   font_data: *mut std::os::raw::c_void,
                                                   glyph: hb::hb_codepoint_t,
                                                   extents: *mut hb::hb_glyph_extents_t,
                                                   closure_data: *mut std::os::raw::c_void)
                                                   -> hb::hb_bool_t
    where F: Fn(&Font, &T, Glyph) -> Option<GlyphExtents>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    match closure(font, font_data, glyph) {
        Some(result) => {
            unsafe { *extents = result };
            1
        }
        None => 0,
    }
}

extern "C" fn rust_get_glyph_contour_point_closure<T, F>(font: *mut hb::hb_font_t,
                                                         font_data: *mut std::os::raw::c_void,
                                                         glyph: hb::hb_codepoint_t,
                                                         point: u32,
                                                         x: *mut Position,
                                                         y: *mut Position,
                                                         closure_data: *mut std::os::raw::c_void)
                                                         -> hb::hb_bool_t
    where F: Fn(&Font, &T, Glyph, u32) -> Option<(Position, Position)>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    if let Some((x_origin, y_origin)) = closure(font, font_data, glyph, point) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_name_closure<T, F>(font: *mut hb::hb_font_t,
                                                font_data: *mut std::os::raw::c_void,
                                                glyph: hb::hb_codepoint_t,
                                                name: *mut std::os::raw::c_char,
                                                size: u32,
                                                closure_data: *mut std::os::raw::c_void)
                                                -> hb::hb_bool_t
    where F: Fn(&Font, &T, Glyph) -> Option<String>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let mut name = unsafe { std::slice::from_raw_parts_mut(name as *mut u8, size as usize) };
    let result = closure(font, font_data, glyph)
        .and_then(|string| CString::new(string).ok())
        .and_then(|cstr| name.write_all(cstr.as_bytes_with_nul()).ok());
    if result.is_some() {
        1
    } else {
        println!("{:?}", (glyph, size));
        name[0] = 0;
        0
    }
}

extern "C" fn rust_get_glyph_from_name_closure<T, F>(font: *mut hb::hb_font_t,
                                                     font_data: *mut std::os::raw::c_void,
                                                     name: *const std::os::raw::c_char,
                                                     size: i32,
                                                     glyph: *mut hb::hb_codepoint_t,
                                                     closure_data: *mut std::os::raw::c_void)
                                                     -> hb::hb_bool_t
    where F: Fn(&Font, &T, &str) -> Option<Glyph>
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { &Font::from_raw_referenced(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let string = match size {
        // `name` is null-terminated
        -1 => unsafe { CStr::from_ptr(name).to_str().ok() },
        // `name` has length = `size`
        i if i >= 1 => unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(name as *const u8, size as usize)).ok()
        },
        _ => None,
    };
    if let Some(result_glyph) = string.and_then(|x| closure(font, font_data, x)) {
        unsafe { *glyph = result_glyph };
        1
    } else {
        0
    }
}

pub struct FontFuncsImpl<T> {
    raw: *mut hb::hb_font_funcs_t,
    _marker: PhantomData<T>,
}

impl<T> FontFuncsImpl<T> {
    pub fn empty() -> FontFuncsImpl<T> {
        let raw = unsafe { hb::hb_font_funcs_get_empty() };
        FontFuncsImpl {
            raw: raw,
            _marker: PhantomData,
        }
    }
}

impl<T: FontFuncs> FontFuncsImpl<T> {
    pub fn from_trait_impl() -> FontFuncsImpl<T> {
        let mut builder = FontFuncsBuilder::new();
        builder.set_trait_impl();
        builder.finish()
    }
}

impl<T> Drop for FontFuncsImpl<T> {
    fn drop(&mut self) {
        unsafe { hb::hb_font_funcs_destroy(self.raw) };
    }
}

unsafe impl<T> Send for FontFuncsImpl<T> {}
unsafe impl<T> Sync for FontFuncsImpl<T> {}

/// A builder struct to create a `FontFuncsImpl`.
///
/// A `FontFuncsBuilder` supports two ways to assign functions to specific font funcs. Either you
/// can set a unique closure per font func or set the font funcs from a type that implements the
/// `FontFuncs` trait.
///
/// # Examples
///
/// Create a `FontFuncsImpl` from individual closures:
///
/// ```
/// use harfbuzz_rs::*;
///
/// let mut builder = FontFuncsBuilder::new();
/// let value = 113;
/// builder.set_font_h_extents_func(|_, _| {
///     Some(FontExtents { ascender: value, ..Default::default() })
/// });
/// let font_funcs: FontFuncsImpl<()> = builder.finish();
/// ```
///
/// Create a `FontFuncsImpl` from a type that implements `FontFuncs`:
///
/// ```
/// use harfbuzz_rs::*;
///
/// // Dummy struct implementing FontFuncs
/// struct MyFontData {
///    value: i32,
/// }
/// impl FontFuncs for MyFontData {
///     fn get_glyph_h_advance(&self, _: &Font, _: Glyph) -> Position {
///         self.value
///     }
///     // implement other trait functions...
/// }
///
/// fn main() {
///     let mut builder = FontFuncsBuilder::new();
///     builder.set_trait_impl();
///     let font_funcs: FontFuncsImpl<MyFontData> = builder.finish();
/// }
/// ```
///
pub struct FontFuncsBuilder<T> {
    raw: *mut hb::hb_font_funcs_t,
    phantom: PhantomData<T>,
}

impl<T> FontFuncsBuilder<T> {
    pub fn new() -> FontFuncsBuilder<T> {
        unsafe {
            let hb_funcs = hb::hb_font_funcs_create();
            FontFuncsBuilder {
                raw: hb_funcs,
                phantom: PhantomData,
            }
        }
    }

    pub fn finish(self) -> FontFuncsImpl<T> {
        unsafe { hb::hb_font_funcs_make_immutable(self.raw) };
        let result = FontFuncsImpl {
            raw: self.raw,
            _marker: PhantomData,
        };
        std::mem::forget(self);
        result
    }

    pub fn set_font_h_extents_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T) -> Option<FontExtents>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_h_extents_func(self.raw,
                                                      Some(rust_get_font_extents_closure::<T, F>),
                                                      Box::into_raw(user_data) as *mut _,
                                                      Some(destroy_box::<F>));
        }
    }

    pub fn set_font_v_extents_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T) -> Option<FontExtents>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_v_extents_func(self.raw,
                                                      Some(rust_get_font_extents_closure::<T, F>),
                                                      Box::into_raw(user_data) as *mut _,
                                                      Some(destroy_box::<F>));
        }
    }

    pub fn set_nominal_glyph_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, char) -> Option<Glyph>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_nominal_glyph_func(self.raw,
                                                     Some(rust_get_nominal_glyph_closure::<T, F>),
                                                     Box::into_raw(user_data) as *mut _,
                                                     Some(destroy_box::<F>));
        }
    }

    pub fn set_variation_glyph_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, char, char) -> Option<Glyph>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_variation_glyph_func(self.raw,
                                                       Some(rust_get_variation_glyph_closure::<T,
                                                                                               F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_h_advance_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Position
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_advance_func(self.raw,
                                                       Some(rust_get_glyph_advance_closure::<T,
                                                                                             F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_v_advance_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Position
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_advance_func(self.raw,
                                                       Some(rust_get_glyph_advance_closure::<T,
                                                                                             F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_h_origin_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Option<(Position, Position)>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_origin_func(self.raw,
                                                      Some(rust_get_glyph_origin_closure::<T, F>),
                                                      Box::into_raw(user_data) as *mut _,
                                                      Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_v_origin_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Option<(Position, Position)>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_origin_func(self.raw,
                                                      Some(rust_get_glyph_origin_closure::<T, F>),
                                                      Box::into_raw(user_data) as *mut _,
                                                      Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_h_kerning_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph, Glyph) -> Position
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_kerning_func(self.raw,
                                                       Some(rust_get_glyph_kerning_closure::<T,
                                                                                             F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_v_kerning_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph, Glyph) -> Position
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_kerning_func(self.raw,
                                                       Some(rust_get_glyph_kerning_closure::<T,
                                                                                             F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_extents_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Option<GlyphExtents>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_extents_func(self.raw,
                                                     Some(rust_get_glyph_extents_closure::<T, F>),
                                                     Box::into_raw(user_data) as *mut _,
                                                     Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_contour_point_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph, u32) -> Option<(Position, Position)>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_contour_point_func(
                self.raw,
                Some(rust_get_glyph_contour_point_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>)
            );
        }
    }

    pub fn set_glyph_name_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, Glyph) -> Option<String>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_name_func(self.raw,
                                                  Some(rust_get_glyph_name_closure::<T, F>),
                                                  Box::into_raw(user_data) as *mut _,
                                                  Some(destroy_box::<F>));
        }
    }

    pub fn set_glyph_from_name_func<F>(&mut self, func: F)
        where F: Fn(&Font, &T, &str) -> Option<Glyph>
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_from_name_func(self.raw,
                                                       Some(rust_get_glyph_from_name_closure::<T,
                                                                                               F>),
                                                       Box::into_raw(user_data) as *mut _,
                                                       Some(destroy_box::<F>));
        }
    }
}

impl<T: FontFuncs> FontFuncsBuilder<T> {
    pub fn set_trait_impl(&mut self) {
        self.set_font_h_extents_func(|font, data| data.get_font_h_extents(font));
        self.set_font_v_extents_func(|font, data| data.get_font_v_extents(font));
        self.set_nominal_glyph_func(|font, data, chr| data.get_nominal_glyph(font, chr));
        self.set_variation_glyph_func(|font, data, chr, var| {
            data.get_variation_glyph(font, chr, var)
        });
        self.set_glyph_h_advance_func(|font, data, glyph| data.get_glyph_h_advance(font, glyph));
        self.set_glyph_v_advance_func(|font, data, glyph| data.get_glyph_v_advance(font, glyph));
        self.set_glyph_h_origin_func(|font, data, glyph| data.get_glyph_h_origin(font, glyph));
        self.set_glyph_v_origin_func(|font, data, glyph| data.get_glyph_v_origin(font, glyph));
        self.set_glyph_h_kerning_func(|font, data, before, after| {
            data.get_glyph_h_kerning(font, before, after)
        });
        self.set_glyph_v_kerning_func(|font, data, before, after| {
            data.get_glyph_v_kerning(font, before, after)
        });
        self.set_glyph_extents_func(|font, data, glyph| data.get_glyph_extents(font, glyph));
        self.set_glyph_contour_point_func(|font, data, glyph, index| {
            data.get_glyph_contour_point(font, glyph, index)
        });
        self.set_glyph_name_func(|font, data, glyph| data.get_glyph_name(font, glyph));
        self.set_glyph_from_name_func(|font, data, name| data.get_glyph_from_name(font, name));
    }
}

impl<T> Drop for FontFuncsBuilder<T> {
    fn drop(&mut self) {
        unsafe { hb::hb_font_funcs_destroy(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use std::default::Default;
    use super::*;
    use face::Face;

    #[derive(Debug)]
    struct MyFontData {
        ascender: i32,
    }
    impl FontFuncs for MyFontData {
        fn get_font_h_extents(&self, _: &Font) -> Option<FontExtents> {
            let extents = FontExtents { ascender: self.ascender, ..Default::default() };
            Some(extents)
        }
    }

    impl Drop for MyFontData {
        fn drop(&mut self) {
            println!("Dropped MyFontData");
        }
    }

    #[test]
    fn test_font_func_trait_impl() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let font = Face::new(&font_bytes[..], 0).create_font();

        let mut subfont = font.create_sub_font();
        let my_funcs = FontFuncsImpl::<MyFontData>::from_trait_impl();
        subfont.set_font_funcs(&my_funcs, MyFontData { ascender: 1212 });

        println!("{:?}", subfont.get_font_h_extents());
        assert_eq!(1212, subfont.get_font_h_extents().unwrap().ascender);
        assert_eq!(34, subfont.get_nominal_glyph('A').unwrap());
    }

    #[test]
    fn test_font_func_closure() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let mut font = Face::new(&font_bytes[..], 0).create_font();

        let mut font_data = MyFontData { ascender: 0 };
        let font_funcs = {
            let mut font_funcs = FontFuncsBuilder::new();
            font_funcs.set_font_h_extents_func(|_, _| {
                Some(FontExtents { ascender: 1313, ..Default::default() })
            });
            font_funcs.set_font_v_extents_func(|_, _| {
                let MyFontData { ascender } = font_data;
                Some(FontExtents { ascender: ascender, ..Default::default() })
            });
            font_funcs.finish()
        };

        font.set_font_funcs(&font_funcs, ());

        for i in 1..1000 {
            font_data.ascender += 1;
            assert_eq!(1313, font.get_font_h_extents().unwrap().ascender);
            assert_eq!(i, font.get_font_v_extents().unwrap().ascender);
        }
    }

    struct GlyphNameFuncProvider {}

    impl FontFuncs for GlyphNameFuncProvider {
        fn get_glyph_name(&self, _: &Font, glyph: Glyph) -> Option<String> {
            Some(format!("My Glyph Code is: {:?}", glyph))
        }

        fn get_glyph_from_name(&self, _: &Font, name: &str) -> Option<Glyph> {
            name.parse().ok()
        }
    }

    #[test]
    fn test_glyph_get_name_func() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let mut font = Face::new(&font_bytes[..], 0).create_font().create_sub_font();
        let glyph_name_funcs = FontFuncsImpl::from_trait_impl();
        font.set_font_funcs(&glyph_name_funcs, GlyphNameFuncProvider {});

        println!("{:?}", font.get_glyph_name(12));
        for i in 1..1000 {
            assert_eq!(format!("My Glyph Code is: {:?}", i),
                       font.get_glyph_name(i).unwrap());
        }
    }

    #[test]
    fn test_glyph_from_name_func() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let mut font = Face::new(&font_bytes[..], 0).create_font().create_sub_font();
        let glyph_name_funcs = FontFuncsImpl::from_trait_impl();
        font.set_font_funcs(&glyph_name_funcs, GlyphNameFuncProvider {});

        for i in 1..1000 {
            assert_eq!(i, font.get_glyph_from_name(&format!("{:?}", i)).unwrap());
        }
    }
}
