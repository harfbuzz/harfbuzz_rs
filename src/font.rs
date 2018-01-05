use hb;
use std;

pub use font_funcs::{FontFuncs, FontFuncsImpl};
use face::Face;
use common::{HarfbuzzObject, HbArc, HbRef, HbBox};

use std::marker::PhantomData;
use std::ffi::CStr;

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

pub(crate) extern "C" fn destroy_box<U>(ptr: *mut std::os::raw::c_void) {
    unsafe { Box::from_raw(ptr as *mut U) };
}

/// A font is the most important concept in harfbuzz.
///
/// A font can be created either as a subfont of an existing font or directly from a `Face` using
/// its `create_font` function.
#[derive(Debug)]
pub struct Font<'a> {
    hb_font: *mut hb::hb_font_t,
    _marker: PhantomData<&'a hb::hb_font_t>,
}

impl<'a> Font<'a> {
    /// Create a new font with a specified `Face`.
    pub fn new<T: Into<HbArc<Face<'a>>>>(face: T) -> HbBox<Self> {
        unsafe {
            let raw_font = hb::hb_font_create(face.into().into_raw());
            // set default font funcs for a completely new font
            hb::hb_ot_font_set_funcs(raw_font);
            HbBox::from_raw(raw_font)
        }
    }

    /// Create a new sub font from the current font that by default inherits its parent font's
    /// face, scale, ppem and font funcs.
    pub fn create_sub_font<T: Into<HbArc<Font<'a>>>>(font: T) -> HbBox<Font<'a>> {
        unsafe { HbBox::from_raw(hb::hb_font_create_sub_font(font.into().as_raw())) }
    }

    pub fn parent(&self) -> HbRef<Font<'a>> {
        unsafe { HbRef::from_raw(hb::hb_font_get_parent(self.hb_font)) }
    }

    pub fn face(&self) -> HbRef<Face<'a>> {
        unsafe { HbRef::from_raw(hb::hb_font_get_face(self.hb_font)) }
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

    pub fn set_font_funcs<F, T, U>(&mut self, funcs: F, font_data: U) -> &mut Font<'a>
        where F: Into<HbArc<FontFuncsImpl<T>>>,
              T: 'a,
              U: Into<MaybeOwned<'a, T>>
    {
        let funcs = funcs.into();
        match font_data.into() {
            MaybeOwned::Owned(font_data) => unsafe {
                let font_data = Box::new(font_data);
                hb::hb_font_set_funcs(self.hb_font,
                                      funcs.as_raw(),
                                      Box::into_raw(font_data) as *mut _,
                                      Some(destroy_box::<T>));
            },
            // TODO: this may be unsafe because we cannot ensure that font_data lives long enough
            MaybeOwned::Ref(font_data) => unsafe {
                hb::hb_font_set_funcs(self.hb_font,
                                      funcs.as_raw(),
                                      font_data as *const T as *mut _,
                                      None);
            },
        }
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

    unsafe fn reference(&self) -> Self {
        let hb_font = hb::hb_font_reference(self.hb_font);
        Font {
            hb_font: hb_font,
            _marker: PhantomData,
        }
    }

    unsafe fn dereference(&self) {
        hb::hb_font_destroy(self.hb_font);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use face::Face;

    #[test]
    fn test_font_reference_counting() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let face = Face::new(&font_bytes[..], 0);
        let font = Font::new(face);
        let font = Font::create_sub_font(font);

        {
            let parent = font.parent();
            println!("{:?}, scale: {:?}, face: {:?}",
                     parent,
                     parent.scale(),
                     parent.face());
        }
        {
            // this could cause a double free if reference counting is incorrect
            let parent = font.parent();
            println!("{:?}, scale: {:?}, face: {:?}",
                     parent,
                     parent.scale(),
                     parent.face());
        }
    }

    #[derive(Debug)]
    struct MyFontData {
        ascender: i32,
    }
    impl FontFuncs for MyFontData {
        fn get_font_h_extents(&self, _: HbRef<Font>) -> Option<FontExtents> {
            let extents = FontExtents {
                ascender: self.ascender,
                ..unsafe { std::mem::zeroed() }
            };
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
        let face = Face::new(&font_bytes[..], 0);
        let font = Font::new(face);

        let mut subfont = Font::create_sub_font(font);
        let my_funcs = FontFuncsImpl::<MyFontData>::from_trait_impl();
        subfont.set_font_funcs(my_funcs, MyFontData { ascender: 1212 });

        println!("{:?}", subfont.get_font_h_extents());
        assert_eq!(1212, subfont.get_font_h_extents().unwrap().ascender);
        assert_eq!(34, subfont.get_nominal_glyph('A').unwrap());
    }

    #[test]
    fn test_font_func_closure() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = Font::new(face);

        let mut font_data = MyFontData { ascender: 0 };
        let mut font_funcs = FontFuncsImpl::new();
        font_funcs.set_font_h_extents_func(|_, _| {
                                               Some(FontExtents {
                                                        ascender: 1313,
                                                        ..unsafe { std::mem::zeroed() }
                                                    })
                                           });
        font_funcs.set_font_v_extents_func(|_, _| {
                                               let MyFontData { ascender } = font_data;
                                               Some(FontExtents {
                                                        ascender: ascender,
                                                        ..unsafe { std::mem::zeroed() }
                                                    })
                                           });

        font.set_font_funcs(font_funcs, ());

        for i in 1..1000 {
            font_data.ascender += 1;
            assert_eq!(1313, font.get_font_h_extents().unwrap().ascender);
            assert_eq!(i, font.get_font_v_extents().unwrap().ascender);
        }
    }

    struct GlyphNameFuncProvider {}

    impl FontFuncs for GlyphNameFuncProvider {
        fn get_glyph_name(&self, _: HbRef<Font>, glyph: Glyph) -> Option<String> {
            Some(format!("My Glyph Code is: {:?}", glyph))
        }

        fn get_glyph_from_name(&self, _: HbRef<Font>, name: &str) -> Option<Glyph> {
            name.parse().ok()
        }
    }

    #[test]
    fn test_glyph_get_name_func() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = Font::new(face);
        let glyph_name_funcs = FontFuncsImpl::from_trait_impl();
        font.set_font_funcs(glyph_name_funcs, GlyphNameFuncProvider {});

        println!("{:?}", font.get_glyph_name(12));
        for i in 1..1000 {
            assert_eq!(format!("My Glyph Code is: {:?}", i),
                       font.get_glyph_name(i).unwrap());
        }
    }

    #[test]
    fn test_glyph_from_name_func() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let face = Face::new(&font_bytes[..], 0);
        let mut font = Font::new(face);
        let glyph_name_funcs = FontFuncsImpl::from_trait_impl();
        font.set_font_funcs(glyph_name_funcs, GlyphNameFuncProvider {});

        for i in 1..1000 {
            assert_eq!(i, font.get_glyph_from_name(&format!("{:?}", i)).unwrap());
        }
    }
}
