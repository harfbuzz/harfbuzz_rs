// Copyright (c) 2018 Manuel Reinhardt
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use font::destroy_box;
use {Font, FontExtents, Glyph, GlyphExtents, HarfbuzzObject, Owned, Position, Shared};

use hb;
use std::os::raw::c_void;

use std;
use std::ffi::{CStr, CString};
use std::fmt;
use std::io::Write;
use std::marker::PhantomData;
use std::panic;

/// This Trait specifies the font callbacks that harfbuzz uses for its shaping. You shouldn't
/// call these functions yourself. They are exposed through the `Font` wrapper.
///
/// No function in this trait needs to be implemented, the default implementations simply return the
/// parent font's data. If a `Font` is created directly from a face, its parent is the
/// empty `Font` which returns null values for every font func.
#[allow(unused_variables)]
pub trait FontFuncs {
    fn get_font_h_extents(&self, font: &Font) -> Option<FontExtents> {
        font.parent()?
            .get_font_h_extents()
            .map(|extents| FontExtents {
                ascender: font.parent_scale_y_distance(|_| extents.ascender),
                descender: font.parent_scale_y_distance(|_| extents.descender),
                line_gap: font.parent_scale_y_distance(|_| extents.line_gap),
                ..extents
            })
    }
    fn get_font_v_extents(&self, font: &Font) -> Option<FontExtents> {
        font.parent()?
            .get_font_v_extents()
            .map(|extents| FontExtents {
                ascender: font.parent_scale_y_distance(|_| extents.ascender),
                descender: font.parent_scale_y_distance(|_| extents.descender),
                line_gap: font.parent_scale_y_distance(|_| extents.line_gap),
                ..extents
            })
    }
    fn get_nominal_glyph(&self, font: &Font, unicode: char) -> Option<Glyph> {
        font.parent()?.get_nominal_glyph(unicode)
    }
    fn get_variation_glyph(
        &self,
        font: &Font,
        unicode: char,
        variation_sel: char,
    ) -> Option<Glyph> {
        font.parent()?.get_variation_glyph(unicode, variation_sel)
    }
    fn get_glyph_h_advance(&self, font: &Font, glyph: Glyph) -> Position {
        font.parent_scale_x_distance(|parent| parent.get_glyph_h_advance(glyph))
    }
    fn get_glyph_v_advance(&self, font: &Font, glyph: Glyph) -> Position {
        font.parent_scale_y_distance(|parent| parent.get_glyph_v_advance(glyph))
    }
    fn get_glyph_h_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent()?
            .get_glyph_h_origin(glyph)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_v_origin(&self, font: &Font, glyph: Glyph) -> Option<(Position, Position)> {
        font.parent()?
            .get_glyph_v_origin(glyph)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_h_kerning(&self, font: &Font, left: Glyph, right: Glyph) -> Position {
        font.parent_scale_x_distance(|parent| parent.get_glyph_h_kerning(left, right))
    }
    fn get_glyph_v_kerning(&self, font: &Font, before: Glyph, after: Glyph) -> Position {
        font.parent_scale_y_distance(|parent| parent.get_glyph_v_kerning(before, after))
    }
    fn get_glyph_extents(&self, font: &Font, glyph: Glyph) -> Option<GlyphExtents> {
        font.parent()?
            .get_glyph_extents(glyph)
            .map(|extents| GlyphExtents {
                x_bearing: font.parent_scale_x_distance(|_| extents.x_bearing),
                y_bearing: font.parent_scale_y_distance(|_| extents.y_bearing),
                width: font.parent_scale_x_distance(|_| extents.width),
                height: font.parent_scale_y_distance(|_| extents.height),
                ..extents
            })
    }
    fn get_glyph_contour_point(
        &self,
        font: &Font,
        glyph: Glyph,
        point_index: u32,
    ) -> Option<(Position, Position)> {
        font.parent()?
            .get_glyph_contour_point(glyph, point_index)
            .map(|x| font.parent_scale_position(x))
    }
    fn get_glyph_name(&self, font: &Font, glyph: Glyph) -> Option<String> {
        font.parent()?.get_glyph_name(glyph)
    }
    fn get_glyph_from_name(&self, font: &Font, name: &str) -> Option<Glyph> {
        font.parent()?.get_glyph_from_name(name)
    }
}

macro_rules! hb_callback {
    ($func_name:ident<$($arg:ident: $datatype:ty),*> -> $ret:ty {
        $(argument $closure_arg:ty => $expr:expr,)*
        return $closure_ret_id:ident: $closure_ret:ty => $ret_expr:expr
    }) => {
        extern "C" fn $func_name<T, F>(
            font: *mut hb::hb_font_t,
            font_data: *mut c_void,
            $(
                $arg: $datatype,
            )*
            closure_data: *mut c_void,
        ) -> $ret where F: Fn(&Font, &T, $($closure_arg),*) -> $closure_ret {
            let catch_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let font_data = unsafe { &*(font_data as *const T) };
                let font = unsafe { Font::from_raw(font) };
                let closure = unsafe { &mut *(closure_data as *mut F) };
                let $closure_ret_id = closure(font, font_data, $($expr),*);
                $ret_expr
            }));
            match catch_result {
                Ok(val) => val,
                Err(_) => {
                    // TODO: Log error
                    Default::default()
                }
            }
        }
    };
}

hb_callback!(
    rust_get_font_extents_closure<metrics: *mut FontExtents> -> hb::hb_bool_t {
        return value: Option<FontExtents> => {
            if let Some(extents) = value {
                unsafe { *metrics = extents };
                1
            } else {
                0
            } 
        }
    }
);

hb_callback!(
    rust_get_nominal_glyph_closure<unicode: hb::hb_codepoint_t, glyph: *mut hb::hb_codepoint_t> -> hb::hb_bool_t {
        argument char => {
            match std::char::from_u32(unicode) {
                Some(character) => character,
                None => return 0
            }
        },
        return result_glyph: Option<Glyph> => {
            if let Some(g) = result_glyph {
                unsafe { *glyph = g }
                1
            } else {
                0
            }
        }
    }
);

hb_callback!(
    rust_get_variation_glyph_closure<
            unicode: hb::hb_codepoint_t, 
            variation_selector: hb::hb_codepoint_t, 
            glyph: *mut hb::hb_codepoint_t> -> hb::hb_bool_t {
        argument char => {
            match std::char::from_u32(unicode) {
                Some(character) => character,
                None => return 0
            }
        },
        argument char => {
            match std::char::from_u32(variation_selector) {
                Some(selector) => selector,
                None => return 0
            }
        },
        return result_glyph: Option<Glyph> => {
            if let Some(g) = result_glyph {
                unsafe { *glyph = g }
                1
            } else {
                0
            }
        }
    }
);

hb_callback!(
    rust_get_glyph_advance_closure<glyph: hb::hb_codepoint_t> -> Position {
        argument Glyph => glyph,
        return pos: Position => pos
    }
);

hb_callback!(
    rust_get_glyph_origin_closure<glyph: hb::hb_codepoint_t, x: *mut Position, y: *mut Position> -> hb::hb_bool_t {
        argument Glyph => glyph,
        return pos: Option<(Position, Position)> => {
            if let Some((x_origin, y_origin)) = pos {
                unsafe {
                    *x = x_origin;
                    *y = y_origin;
                }
                1
            } else {
                0
            }
        }
    }
);

hb_callback!(
    rust_get_glyph_kerning_closure<before: hb::hb_codepoint_t, after: hb::hb_codepoint_t> -> Position {
        argument Glyph => before,
        argument Glyph => after,
        return pos: Position => pos
    }
);

hb_callback!(
    rust_get_glyph_extents_closure<glyph: hb::hb_codepoint_t, extents: *mut hb::hb_glyph_extents_t> -> hb::hb_bool_t {
        argument Glyph => glyph,
        return value: Option<GlyphExtents> => {
            match value {
                Some(result) => {
                    unsafe { *extents = result };
                    1
                }
                None => 0,
            }
        }
    }
);

hb_callback!(
    rust_get_glyph_contour_point_closure<glyph: hb::hb_codepoint_t, point: u32, x: *mut Position, y: *mut Position> -> hb::hb_bool_t {
        argument Glyph => glyph,
        argument u32 => point,
        return value: Option<(Position, Position)> => {
            match value {
                Some((x_origin, y_origin)) => unsafe {
                    *x = x_origin;
                    *y = y_origin;
                    1
                },
                None => 0
            }
        }
    }
);

hb_callback!(
    rust_get_glyph_name_closure<glyph: hb::hb_codepoint_t, name: *mut std::os::raw::c_char, size: u32> -> hb::hb_bool_t {
        argument Glyph => glyph,
        return value: Option<String> => {
            let mut name = unsafe { std::slice::from_raw_parts_mut(name as *mut u8, size as usize) };
            let result = value
                .and_then(|string| CString::new(string).ok())
                .and_then(|cstr| name.write_all(cstr.as_bytes_with_nul()).ok());
            if result.is_some() {
                1
            } else {
                name[0] = 0;
                0
            }
        }
    }
);

hb_callback!(
    rust_get_glyph_from_name_closure<name: *const std::os::raw::c_char, size: i32, glyph: *mut hb::hb_codepoint_t> -> hb::hb_bool_t {
        argument &str => {
            let string = match size {
                // `name` is null-terminated
                -1 => unsafe { CStr::from_ptr(name).to_str().ok() },
                // `name` has length = `size`
                i if i >= 1 => unsafe {
                    std::str::from_utf8(std::slice::from_raw_parts(name as *const u8, size as usize)).ok()
                },
                _ => None,
            };
            match string {
                Some(string) => string,
                None => return 0,
            }
        },
        return result_glyph: Option<Glyph> => {
            if let Some(g) = result_glyph {
                unsafe { *glyph = g }
                1
            } else {
                0
            }
        }
    }
);

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
/// ```ignore
/// use harfbuzz_rs::*;
/// use std::mem;
///
/// let mut ffuncs: Owned<FontFuncsImpl<()>> = FontFuncsImpl::new();
/// let value = 113;
/// ffuncs.set_font_h_extents_func(|_, _| {
///     Some(FontExtents { ascender: value, .. unsafe { mem::zeroed() } })
/// });
/// ```
///
/// Create a `FontFuncsImpl` from a type that implements `FontFuncs`:
///
/// ```ignore
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
///     // implementations of other functions...
/// }
///
/// fn main() {
///     let font_funcs: Owned<FontFuncsImpl<MyFontData>> = FontFuncsImpl::from_trait_impl();
/// }
/// ```
///
/// After creating font funcs they can be set on a font to change the font implementation that will
/// be used by HarfBuzz while shaping.
///
#[repr(C)]
pub struct FontFuncsImpl<T> {
    _raw: hb::hb_font_funcs_t,
    _marker: PhantomData<T>,
}

impl<T> FontFuncsImpl<T> {
    /// Returns an empty `FontFuncsImpl`. Every font callback of the returned `FontFuncsImpl` gives
    /// a null value regardless of its input.
    #[allow(unused)]
    pub fn empty() -> Shared<FontFuncsImpl<T>> {
        let raw = unsafe { hb::hb_font_funcs_get_empty() };
        unsafe { Shared::from_raw(raw) }
    }
}

impl<T: FontFuncs> FontFuncsImpl<T> {
    /// Create a new `FontFuncsImpl` from the `FontFuncs`-trait implementation of `T`.
    ///
    /// # Examples
    ///
    /// Supposing `MyFontData` is a struct that implements `FontFuncs`.
    ///
    /// ```ignore
    /// use harfbuzz_rs::*;
    ///
    /// # // Dummy struct implementing FontFuncs
    /// # struct MyFontData {
    /// #    value: i32,
    /// # }
    /// # impl FontFuncs for MyFontData {
    /// #     fn get_glyph_h_advance(&self, _: &Font, _: Glyph) -> Position {
    /// #         self.value
    /// #     }
    /// #     // implement other trait functions...
    /// # }
    /// #
    /// # fn main() {
    /// let font_funcs: Owned<FontFuncsImpl<MyFontData>> = FontFuncsImpl::from_trait_impl();
    /// # }
    /// ```
    ///
    pub fn from_trait_impl() -> Owned<FontFuncsImpl<T>> {
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
    pub fn new() -> Owned<FontFuncsImpl<T>> {
        unsafe { Owned::from_raw(hb::hb_font_funcs_create()) }
    }

    pub fn set_font_h_extents_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T) -> Option<FontExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_h_extents_func(
                self.as_raw(),
                Some(rust_get_font_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_font_v_extents_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T) -> Option<FontExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_font_v_extents_func(
                self.as_raw(),
                Some(rust_get_font_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_nominal_glyph_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, char) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_nominal_glyph_func(
                self.as_raw(),
                Some(rust_get_nominal_glyph_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_variation_glyph_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, char, char) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_variation_glyph_func(
                self.as_raw(),
                Some(rust_get_variation_glyph_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_advance_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_advance_func(
                self.as_raw(),
                Some(rust_get_glyph_advance_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_advance_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_advance_func(
                self.as_raw(),
                Some(rust_get_glyph_advance_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_origin_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_origin_func(
                self.as_raw(),
                Some(rust_get_glyph_origin_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_origin_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_origin_func(
                self.as_raw(),
                Some(rust_get_glyph_origin_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_h_kerning_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_h_kerning_func(
                self.as_raw(),
                Some(rust_get_glyph_kerning_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_v_kerning_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph, Glyph) -> Position,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_v_kerning_func(
                self.as_raw(),
                Some(rust_get_glyph_kerning_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_extents_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Option<GlyphExtents>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_extents_func(
                self.as_raw(),
                Some(rust_get_glyph_extents_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_contour_point_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph, u32) -> Option<(Position, Position)>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_contour_point_func(
                self.as_raw(),
                Some(rust_get_glyph_contour_point_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_name_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, Glyph) -> Option<String>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_name_func(
                self.as_raw(),
                Some(rust_get_glyph_name_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_glyph_from_name_func<F>(&mut self, func: F)
    where
        F: Fn(&Font, &T, &str) -> Option<Glyph>,
    {
        let user_data = Box::new(func);
        unsafe {
            hb::hb_font_funcs_set_glyph_from_name_func(
                self.as_raw(),
                Some(rust_get_glyph_from_name_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }
}

impl<T> fmt::Debug for FontFuncsImpl<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FontFuncsImpl").finish()
    }
}

unsafe impl<T> HarfbuzzObject for FontFuncsImpl<T> {
    type Raw = hb::hb_font_funcs_t;

    unsafe fn reference(&self) {
        hb::hb_font_funcs_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb::hb_font_funcs_destroy(self.as_raw())
    }
}

unsafe impl<T> Send for FontFuncsImpl<T> {}
unsafe impl<T> Sync for FontFuncsImpl<T> {}
