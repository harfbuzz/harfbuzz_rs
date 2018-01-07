// Copyright (c) 2018 Manuel Reinhardt
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use {Font, FontExtents, Glyph, GlyphExtents, HarfbuzzObject, HbArc, HbBox, HbRef, Position};
use font::destroy_box;

use libc::c_void;
use hb;

use std;
use std::marker::PhantomData;
use std::ffi::{CStr, CString};
use std::io::Write;

/// This Trait specifies the font callbacks that harfbuzz uses for its shaping. You shouldn't
/// call these functions yourself. They are exposed through the `Font` wrapper.
///
/// No function in this trait needs to be implemented, the default implementations simply return the
/// parent font's implementation. If a `Font` is created directly from a face, its parent is the
/// empty `Font` which returns null values for every font func.
#[allow(unused_variables)]
pub trait FontFuncs {
    fn get_font_h_extents(&self, font: HbRef<Font>) -> Option<FontExtents> {
        font.parent().get_font_h_extents().map(|extents| {
            FontExtents {
                ascender: font.parent_scale_y_distance(extents.ascender),
                descender: font.parent_scale_y_distance(extents.descender),
                line_gap: font.parent_scale_y_distance(extents.line_gap),
                ..extents
            }
        })
    }
    fn get_font_v_extents(&self, font: HbRef<Font>) -> Option<FontExtents> {
        font.parent().get_font_v_extents().map(|extents| {
            FontExtents {
                ascender: font.parent_scale_y_distance(extents.ascender),
                descender: font.parent_scale_y_distance(extents.descender),
                line_gap: font.parent_scale_y_distance(extents.line_gap),
                ..extents
            }
        })
    }
    fn get_nominal_glyph(&self, font: HbRef<Font>, unicode: char) -> Option<Glyph> {
        font.parent().get_nominal_glyph(unicode)
    }
    fn get_variation_glyph(
        &self,
        font: HbRef<Font>,
        unicode: char,
        variation_sel: char,
    ) -> Option<Glyph> {
        font.parent().get_variation_glyph(unicode, variation_sel)
    }
    fn get_glyph_h_advance(&self, font: HbRef<Font>, glyph: Glyph) -> Position {
        font.parent_scale_x_distance(font.parent().get_glyph_h_advance(glyph))
    }
    fn get_glyph_v_advance(&self, font: HbRef<Font>, glyph: Glyph) -> Position {
        font.parent_scale_y_distance(font.parent().get_glyph_v_advance(glyph))
    }
    fn get_glyph_h_origin(&self, font: HbRef<Font>, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent()
            .get_glyph_h_origin(glyph)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_v_origin(&self, font: HbRef<Font>, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent()
            .get_glyph_v_origin(glyph)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_h_kerning(&self, font: HbRef<Font>, left: Glyph, right: Glyph) -> Position {
        font.parent_scale_x_distance(font.parent().get_glyph_h_kerning(left, right))
    }
    fn get_glyph_v_kerning(&self, font: HbRef<Font>, before: Glyph, after: Glyph) -> Position {
        font.parent_scale_y_distance(font.parent().get_glyph_v_kerning(before, after))
    }
    fn get_glyph_extents(&self, font: HbRef<Font>, glyph: Glyph) -> Option<GlyphExtents> {
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
    fn get_glyph_contour_point(
        &self,
        font: HbRef<Font>,
        glyph: Glyph,
        point_index: u32,
    ) -> Option<(Position, Position)> {
        font.parent()
            .get_glyph_contour_point(glyph, point_index)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_name(&self, font: HbRef<Font>, glyph: Glyph) -> Option<String> {
        font.parent().get_glyph_name(glyph)
    }
    fn get_glyph_from_name(&self, font: HbRef<Font>, name: &str) -> Option<Glyph> {
        font.parent().get_glyph_from_name(name)
    }
}


extern "C" fn rust_get_font_extents_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    metrics: *mut FontExtents,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T) -> Option<FontExtents>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    let result = closure(font, font_data);

    if let Some(extents) = result {
        unsafe { *metrics = extents };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_nominal_glyph_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    unicode: hb::hb_codepoint_t,
    glyph: *mut hb::hb_codepoint_t,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, char) -> Option<Glyph>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
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

extern "C" fn rust_get_variation_glyph_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    unicode: hb::hb_codepoint_t,
    variation_selector: hb::hb_codepoint_t,
    glyph: *mut hb::hb_codepoint_t,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, char, char) -> Option<Glyph>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
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

extern "C" fn rust_get_glyph_advance_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    glyph: hb::hb_codepoint_t,
    closure_data: *mut c_void,
) -> Position
where
    F: Fn(HbRef<Font>, &T, Glyph) -> Position,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    closure(font, font_data, glyph)
}

extern "C" fn rust_get_glyph_origin_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    glyph: hb::hb_codepoint_t,
    x: *mut Position,
    y: *mut Position,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, Glyph) -> Option<(Position, Position)>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    if let Some((x_origin, y_origin)) = closure(font, font_data, glyph) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_kerning_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    before: hb::hb_codepoint_t,
    after: hb::hb_codepoint_t,
    closure_data: *mut c_void,
) -> Position
where
    F: Fn(HbRef<Font>, &T, Glyph, Glyph) -> Position,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    closure(font, font_data, before, after)
}

extern "C" fn rust_get_glyph_extents_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    glyph: hb::hb_codepoint_t,
    extents: *mut hb::hb_glyph_extents_t,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, Glyph) -> Option<GlyphExtents>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    match closure(font, font_data, glyph) {
        Some(result) => {
            unsafe { *extents = result };
            1
        }
        None => 0,
    }
}

extern "C" fn rust_get_glyph_contour_point_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    glyph: hb::hb_codepoint_t,
    point: u32,
    x: *mut Position,
    y: *mut Position,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, Glyph, u32) -> Option<(Position, Position)>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
    let closure = unsafe { &mut *(closure_data as *mut F) };
    if let Some((x_origin, y_origin)) = closure(font, font_data, glyph, point) {
        unsafe { *x = x_origin };
        unsafe { *y = y_origin };
        1
    } else {
        0
    }
}

extern "C" fn rust_get_glyph_name_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    glyph: hb::hb_codepoint_t,
    name: *mut std::os::raw::c_char,
    size: u32,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, Glyph) -> Option<String>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
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

extern "C" fn rust_get_glyph_from_name_closure<T, F>(
    font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    name: *const std::os::raw::c_char,
    size: i32,
    glyph: *mut hb::hb_codepoint_t,
    closure_data: *mut c_void,
) -> hb::hb_bool_t
where
    F: Fn(HbRef<Font>, &T, &str) -> Option<Glyph>,
{
    let font_data = unsafe { &*(font_data as *const T) };
    let font = unsafe { HbRef::from_raw(font) };
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

/// A `FontFuncsImpl` contains implementations of the font callbacks that harfbuzz uses.
///
/// It supports two ways to assign functions to specific font funcs. Either you
/// can set a unique closure per font func or set the font funcs from a type that implements the
/// `FontFuncs` trait using the `from_trait_impl` constructor.
///
/// # Examples
///
/// Create a `FontFuncsImpl` from individual closures:
///
/// ```
/// use harfbuzz_rs::*;
/// use std::mem;
///
/// let mut ffuncs: HbBox<FontFuncsImpl<()>> = FontFuncsImpl::new();
/// let value = 113;
/// ffuncs.set_font_h_extents_func(|_, _| {
///     Some(FontExtents { ascender: value, .. unsafe { mem::zeroed() } })
/// });
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
///     fn get_glyph_h_advance(&self, _: HbRef<Font>, _: Glyph) -> Position {
///         self.value
///     }
///     // implementations of other functions...
/// }
///
/// fn main() {
///     let font_funcs: HbBox<FontFuncsImpl<MyFontData>> = FontFuncsImpl::from_trait_impl();
/// }
/// ```
///
/// After creating font funcs they can be set on a font to change the font implementation that will
/// be used by HarfBuzz while shaping.
///
pub struct FontFuncsImpl<T> {
    raw: *mut hb::hb_font_funcs_t,
    _marker: PhantomData<T>,
}

impl<T> FontFuncsImpl<T> {
    /// Returns an empty `FontFuncsImpl`. Every font callback of the returned `FontFuncsImpl` gives
    /// a null value regardless of its input.
    pub fn empty() -> HbArc<FontFuncsImpl<T>> {
        let raw = unsafe { hb::hb_font_funcs_get_empty() };
        unsafe { HbArc::from_raw(raw) }
    }
}

impl<T: FontFuncs> FontFuncsImpl<T> {
    /// Create a new `FontFuncsImpl` from the `FontFuncs`-trait implementation of `T`.
    ///
    /// # Examples
    ///
    /// Supposing `MyFontData` is a struct that implements `FontFuncs`.
    ///
    /// ```
    /// use harfbuzz_rs::*;
    ///
    /// # // Dummy struct implementing FontFuncs
    /// # struct MyFontData {
    /// #    value: i32,
    /// # }
    /// # impl FontFuncs for MyFontData {
    /// #     fn get_glyph_h_advance(&self, _: HbRef<Font>, _: Glyph) -> Position {
    /// #         self.value
    /// #     }
    /// #     // implement other trait functions...
    /// # }
    /// #
    /// # fn main() {
    /// let font_funcs: HbBox<FontFuncsImpl<MyFontData>> = FontFuncsImpl::from_trait_impl();
    /// # }
    /// ```
    ///
    pub fn from_trait_impl() -> HbBox<FontFuncsImpl<T>> {
        let mut ffuncs = FontFuncsImpl::new();
        ffuncs.set_trait_impl();
        ffuncs
    }

    fn set_trait_impl(&mut self) {
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

impl<T> FontFuncsImpl<T> {
    pub fn new() -> HbBox<FontFuncsImpl<T>> {
        unsafe { HbBox::from_raw(hb::hb_font_funcs_create()) }
    }

    pub fn set_font_h_extents_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T) -> Option<FontExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_h_extents_func(
                self.raw,
                Some(rust_get_font_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_font_v_extents_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T) -> Option<FontExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_v_extents_func(
                self.raw,
                Some(rust_get_font_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_nominal_glyph_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, char) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_nominal_glyph_func(
                self.raw,
                Some(rust_get_nominal_glyph_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_variation_glyph_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, char, char) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_variation_glyph_func(
                self.raw,
                Some(rust_get_variation_glyph_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_advance_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_advance_func(
                self.raw,
                Some(rust_get_glyph_advance_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_advance_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_advance_func(
                self.raw,
                Some(rust_get_glyph_advance_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_origin_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_origin_func(
                self.raw,
                Some(rust_get_glyph_origin_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_origin_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_origin_func(
                self.raw,
                Some(rust_get_glyph_origin_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_kerning_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_kerning_func(
                self.raw,
                Some(rust_get_glyph_kerning_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_kerning_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_kerning_func(
                self.raw,
                Some(rust_get_glyph_kerning_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_extents_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Option<GlyphExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_extents_func(
                self.raw,
                Some(rust_get_glyph_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_contour_point_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph, u32) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_contour_point_func(
                self.raw,
                Some(rust_get_glyph_contour_point_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_name_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, Glyph) -> Option<String>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_name_func(
                self.raw,
                Some(rust_get_glyph_name_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_from_name_func<F>(&mut self, func: F)
    where
        F: Fn(HbRef<Font>, &T, &str) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_from_name_func(
                self.raw,
                Some(rust_get_glyph_from_name_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }
}

impl<T> HarfbuzzObject for FontFuncsImpl<T> {
    type Raw = *mut hb::hb_font_funcs_t;
    unsafe fn from_raw(val: Self::Raw) -> Self {
        FontFuncsImpl {
            raw: val,
            _marker: PhantomData,
        }
    }

    fn as_raw(&self) -> Self::Raw {
        self.raw
    }

    unsafe fn reference(&self) -> Self {
        hb::hb_font_funcs_reference(self.raw);
        FontFuncsImpl {
            raw: self.raw,
            _marker: PhantomData,
        }
    }

    unsafe fn dereference(&self) {
        hb::hb_font_funcs_destroy(self.raw)
    }
}

unsafe impl<T> Send for FontFuncsImpl<T> {}
unsafe impl<T> Sync for FontFuncsImpl<T> {}
