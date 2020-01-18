use crate::hb;
use std;
use std::ptr::NonNull;

use std::os::raw::c_void;

use crate::common::{HarfbuzzObject, Owned, Shared};
use crate::face::Face;
pub use crate::font_funcs::FontFuncs;
use crate::font_funcs::FontFuncsImpl;

use std::ffi::CStr;
use std::marker::PhantomData;

pub type Glyph = u32;
pub type Position = hb::hb_position_t;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct FontExtents {
    pub ascender: Position,
    pub descender: Position,
    pub line_gap: Position,
    pub(crate) reserved: [Position; 9],
}

impl FontExtents {
    pub fn new(ascender: Position, descender: Position, line_gap: Position) -> FontExtents {
        FontExtents {
            ascender,
            descender,
            line_gap,
            ..Default::default()
        }
    }

    pub fn into_raw(self) -> hb::hb_font_extents_t {
        unsafe { std::mem::transmute(self) }
    }

    pub fn from_raw(raw: hb::hb_font_extents_t) -> FontExtents {
        unsafe { std::mem::transmute(raw) }
    }
}

pub type GlyphExtents = hb::hb_glyph_extents_t;

pub(crate) extern "C" fn destroy_box<U>(ptr: *mut c_void) {
    unsafe { Box::from_raw(ptr as *mut U) };
}

/// A type representing a single font (i.e. a specific combination of typeface
/// and typesize).
///
/// It safely wraps `hb_font_t`.
///
/// # Font Funcs
///
/// A font is one of the most important structures in harfbuzz. It coordinates
/// how glyph information is accessed during shaping. This is done through
/// so-called font funcs.
///
/// You can manually define new font funcs according to your needs, in most
/// cases though the default font funcs provided by HarfBuzz will suffice. In
/// that case the creation of a usable font amounts to calling the `Font::new`
/// constructor with the desired `Face`.
///
/// # Parents and Children
///
/// Every font except the empty font has a parent font. If a font does not have
/// some font func set, it will automatically use the parent's implementation of
/// that font func. This behavior is useful to effectively "subclass" font
/// objects to use different font function implementations for some font funcs
/// while reusing the parent's implementation for the remaining funcs.
///
/// Since every font created by `Font::new` by default uses HarfBuzz's internal
/// font funcs they can be used as a fallback mechanism by only customizing the
/// font funcs of a sub-font.
///
/// # Examples
///
/// Create a simple font from a `Face` using the default font funcs:
///
/// ```
/// use harfbuzz_rs::*;
/// # use std::path::PathBuf;
/// # let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// # path.push("testfiles/SourceSansVariable-Roman.ttf");
/// let face = Face::from_file(path, 0).expect("Error reading font file.");
/// let font = Font::new(face);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Font<'a> {
    raw: NonNull<hb::hb_font_t>,
    marker: PhantomData<&'a hb::hb_font_t>,
}

impl<'a> Font<'a> {
    /// Create a new font from the specified `Face`.
    ///
    /// This is the default constructor of `Font`. In many cases it is the best
    /// choice if you simply want to shape some text.
    ///
    /// The default parent of a font created by this function is the empty font.
    ///
    /// # Font Functions
    ///
    /// The font returned by this function uses the font funcs that come with
    /// HarfBuzz for OpenType Fonts. The font funcs can be overwritten using
    /// `Font::set_font_funcs`.
    ///
    /// # Errors
    ///
    /// If for some reason no valid font can be constructed this function will
    /// return the empty font.
    pub fn new<T: Into<Shared<Face<'a>>>>(face: T) -> Owned<Self> {
        unsafe {
            let face = face.into();
            let raw_font = hb::hb_font_create(face.as_raw());
            // set default font funcs for a completely new font
            hb::hb_ot_font_set_funcs(raw_font);
            Owned::from_raw(raw_font)
        }
    }

    /// Returns an empty font.
    ///
    /// This can be useful when you need a dummy font for whatever reason. Any
    /// function you call on the empty font will return some reasonable default
    /// value. An empty font is the only font whose `.parent()` method returns
    /// `None`.
    pub fn empty() -> Owned<Self> {
        unsafe {
            let raw_font = hb::hb_font_get_empty();
            Owned::from_raw(raw_font)
        }
    }

    /// Create a new sub font from the current font that by default inherits its
    /// parent font's face, scale, ppem and font funcs.
    ///
    /// The sub-font's parent will be the font on which this method is called.
    ///
    /// Creating sub-fonts is especially useful if you want to overwrite some of
    /// the font funcs of an already existing font.
    ///
    /// # Examples
    /// ```
    /// use harfbuzz_rs::*;
    /// # use std::path::PathBuf;
    /// # let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # path.push("testfiles/SourceSansVariable-Roman.ttf");
    /// let face = Face::from_file(path, 0).expect("Error reading font file.");
    /// let font = Font::new(face).to_shared();
    ///
    /// let sub_font = Font::create_sub_font(font.clone());
    /// // we know that sub_font has a parent
    /// assert_eq!(sub_font.parent().unwrap(), font);
    /// ```
    pub fn create_sub_font<T: Into<Shared<Self>>>(font: T) -> Owned<Self> {
        unsafe { Owned::from_raw(hb::hb_font_create_sub_font(font.into().as_raw())) }
    }

    /// Returns a shared pointer to the parent font.
    ///
    /// If `self` is the empty font it returns `None`.
    ///
    /// # See also
    ///
    /// [`create_sub_font`](./struct.Font.html#method.create_sub_font)
    ///
    /// # Examples
    ///
    /// The empty font (and only it) has no parent:
    ///
    /// ```
    /// use harfbuzz_rs::Font;
    ///
    /// let font = Font::empty();
    /// assert_eq!(font.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<Shared<Self>> {
        unsafe {
            let parent = hb::hb_font_get_parent(self.as_raw());
            if parent.is_null() {
                // hb_font_get_parent returns null-ptr if called on the empty font.
                None
            } else {
                Some(Shared::from_raw_ref(parent))
            }
        }
    }

    /// Returns a shared pointer to the face from which this font was created.
    pub fn face(&self) -> Shared<Face<'a>> {
        unsafe { Shared::from_raw_ref(hb::hb_font_get_face(self.as_raw())) }
    }

    /// Returns the EM scale of the font.
    pub fn scale(&self) -> (i32, i32) {
        let mut result = (0i32, 0i32);
        unsafe { hb::hb_font_get_scale(self.as_raw(), &mut result.0, &mut result.1) };
        result
    }

    /// Sets the EM scale of the font.
    pub fn set_scale(&mut self, x: i32, y: i32) {
        unsafe { hb::hb_font_set_scale(self.as_raw(), x, y) };
    }

    pub fn ppem(&self) -> (u32, u32) {
        let mut result = (0u32, 0u32);
        unsafe { hb::hb_font_get_ppem(self.as_raw(), &mut result.0, &mut result.1) };
        result
    }

    pub fn set_ppem(&mut self, x: u32, y: u32) {
        unsafe { hb::hb_font_set_ppem(self.as_raw(), x, y) };
    }

    /// Sets the font functions that this font will have from a value that
    /// implements [`FontFuncs`](./font_funcs/trait.FontFuncs.html).
    pub fn set_font_funcs<FuncsType>(&mut self, funcs: FuncsType)
    where
        FuncsType: 'a + Send + Sync + FontFuncs,
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
    }

    // scale from parent font
    pub(crate) fn parent_scale_x_distance(&self, f: impl Fn(&Font<'_>) -> Position) -> Position {
        let x_scale = self.scale().0;
        if let Some(parent) = self.parent() {
            let parent_x_scale = parent.scale().0;

            if parent_x_scale != x_scale {
                (f(&parent) as i64 * x_scale as i64 / parent_x_scale as i64) as Position
            } else {
                f(&parent)
            }
        } else {
            0
        }
    }

    // scale from parent font
    pub(crate) fn parent_scale_y_distance(&self, f: impl Fn(&Font<'_>) -> Position) -> Position {
        let y_scale = self.scale().0;
        if let Some(parent) = self.parent() {
            let parent_y_scale = parent.scale().0;

            if parent_y_scale != y_scale {
                (f(&parent) as i64 * y_scale as i64 / parent_y_scale as i64) as Position
            } else {
                f(&parent)
            }
        } else {
            0
        }
    }

    // scale from parent font
    pub(crate) fn parent_scale_position(&self, v: (Position, Position)) -> (Position, Position) {
        (
            self.parent_scale_x_distance(|_| v.0),
            self.parent_scale_y_distance(|_| v.1),
        )
    }

    pub fn get_font_h_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = FontExtents::default();
            let result = hb::hb_font_get_h_extents(
                self.as_raw(),
                &mut extents as *mut FontExtents as *mut _,
            );
            if result == 1 {
                Some(extents)
            } else {
                None
            }
        }
    }

    pub fn get_font_v_extents(&self) -> Option<FontExtents> {
        unsafe {
            let mut extents = std::mem::zeroed::<FontExtents>();
            let result = hb::hb_font_get_v_extents(
                self.as_raw(),
                &mut extents as *mut FontExtents as *mut _,
            );
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

    /// Get the horizontal advance width of a glyph.
    pub fn get_glyph_h_advance(&self, glyph: Glyph) -> Position {
        unsafe { hb::hb_font_get_glyph_h_advance(self.as_raw(), glyph) }
    }

    /// Get the vertical advance width of a glyph.
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

    pub fn get_glyph_extents(&self, glyph: Glyph) -> Option<GlyphExtents> {
        unsafe {
            let mut extents = std::mem::zeroed::<GlyphExtents>();
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

unsafe impl<'a> Send for Font<'a> {}
unsafe impl<'a> Sync for Font<'a> {}

unsafe impl<'a> HarfbuzzObject for Font<'a> {
    type Raw = hb::hb_font_t;

    unsafe fn from_raw(raw: *const Self::Raw) -> Self {
        Font {
            raw: NonNull::new_unchecked(raw as *mut _),
            marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut Self::Raw {
        self.raw.as_ptr()
    }

    unsafe fn reference(&self) {
        hb::hb_font_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb::hb_font_destroy(self.as_raw());
    }
}

impl<'a> Default for Owned<Font<'a>> {
    fn default() -> Self {
        Font::empty()
    }
}

impl<'a> Default for Shared<Font<'a>> {
    fn default() -> Self {
        Font::empty().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::assert_memory_layout_equal;

    #[test]
    fn test_font_extents_layout() {
        assert_memory_layout_equal::<FontExtents, hb::hb_font_extents_t>()
    }
}
