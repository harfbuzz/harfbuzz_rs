//! `harfbuzz_rs` is a high-level interface to HarfBuzz, exposing its most important functionality
//! in a safe manner using Rust.
//!
//! # What is HarfBuzz?
//! HarfBuzz is a library for performing complex text layout. It does not perform any drawing. This
//! is quite a low-level operation. If you want to simply draw some text on the screen choose
//! another library. However if you want to build a library for drawing text on some canvas or
//! need a lot of control on advanced text layout then this is the right library to use.
//!
//! # Getting Started
//!
//! To shape a simple string of text you just create a `Font` from a font file, fill a `Buffer`
//! with some text and call the `shape` function.
//!
//! ```
//! use harfbuzz_rs::*;
//!
//! # fn try_main() -> Result<(), std::io::Error> {
//!
//! let path = "path/to/some/font_file.otf";
//! let index = 0; //< face index in the font file
//! # let path = "testfiles/SourceSansVariable-Roman.ttf";
//! let face = Face::from_file(path, index)?;
//! let mut font = Font::new(face);
//!
//! let buffer = UnicodeBuffer::new().add_str("Hello World!");
//! let output = shape(&font, buffer, &[]);
//!
//! // The results of the shaping operation are stored in the `output` buffer.
//!
//! let positions = output.get_glyph_positions();
//! let infos = output.get_glyph_infos();
//!
//! # assert_eq!(positions.len(), 12);
//! assert_eq!(positions.len(), infos.len());
//!
//! // iterate over the shaped glyphs
//! for (position, info) in positions.iter().zip(infos) {
//!     let gid = info.codepoint;
//!     let cluster = info.cluster;
//!     let x_advance = position.x_advance;
//!     let x_offset = position.x_offset;
//!     let y_offset = position.y_offset;
//!
//!     // Here you would usually draw the glyphs.
//!     println!("gid{:?}={:?}@{:?},{:?}+{:?}", gid, cluster, x_advance, x_offset, y_offset);
//! }
//!
//! # Ok(())
//! # }
//! #
//! # try_main().unwrap();
//! ```
//! This should print out something similar to the following:
//!
//! ```text
//! gid41=0@741,0+0
//! gid70=1@421,0+0
//! gid77=2@258,0+0
//! gid77=3@253,0+0
//! gid80=4@510,0+0
//! gid1=5@227,0+0
//! gid56=6@874,0+0
//! gid80=7@498,0+0
//! gid83=8@367,0+0
//! gid77=9@253,0+0
//! gid69=10@528,0+0
//! gid2=11@276,0+0
//! ```
#![deny(missing_debug_implementations)]

/// Reexported `harfbuzz_sys` crate to directly access the C API whenever no
/// adequate wrapper is provided.
// This will hopefully not cause backwards compability concerns since harfbuzz
// tries to be backwards compatible.
#[macro_use]
extern crate bitflags;

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(deref_nullptr)]
#[allow(dead_code)]
mod bindings;
mod blob;
mod buffer;
mod common;
pub mod draw_funcs;
mod face;
mod font;
pub mod font_funcs;

#[cfg(feature = "rusttype")]
pub mod rusttype;

use bindings::hb_feature_t;
use bindings::hb_shape;
use bindings::hb_variation_t;

pub use crate::blob::*;
pub use crate::buffer::*;
pub use crate::common::*;
pub use crate::face::*;
pub use crate::font::*;

use std::ops::{Bound, RangeBounds};
use std::os::raw::c_uint;

pub(crate) fn start_end_range(range: impl RangeBounds<usize>) -> (c_uint, c_uint) {
    // We have to do careful bounds checking since c_uint may be of
    // different sizes on different platforms. We do assume that
    // sizeof(usize) >= sizeof(c_uint).
    const MAX_UINT: usize = c_uint::max_value() as usize;
    let start = match range.start_bound() {
        Bound::Included(&included) => included.min(MAX_UINT) as c_uint,
        Bound::Excluded(&excluded) => excluded.min(MAX_UINT - 1) as c_uint + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&included) => included.saturating_add(1).min(MAX_UINT) as c_uint,
        Bound::Excluded(&excluded) => excluded.min(MAX_UINT) as c_uint,
        Bound::Unbounded => c_uint::max_value(),
    };
    (start, end)
}

/// A variation selector which can be applied to a specific font.
///
/// To use OpenType variations when shaping see the documentation of [`Font`].
///
/// # Fields
///
/// - `tag`: The OpenType tag of the variation.
/// - `value`: Some OpenType variant accept different values to change
///   their behaviour.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Variation(hb_variation_t);

impl Variation {
    /// Create a new Variation with provided `tag` and `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use harfbuzz_rs::Variation;
    /// Variation::new(b"wght", 800.0);
    /// ```
    pub fn new(tag: impl Into<Tag>, value: f32) -> Variation {
        Variation(hb_variation_t {
            tag: tag.into().0,
            value,
        })
    }

    /// Returns the `tag` of the variation.
    pub fn tag(&self) -> Tag {
        Tag(self.0.tag)
    }

    /// Returns the value of the variation.
    pub fn value(&self) -> f32 {
        self.0.value
    }
}

/// A feature tag with an accompanying range specifying on which subslice of
/// `shape`s input it should be applied.
///
/// You can pass a slice of `Feature`s to `shape` that will be activated for the
/// corresponding slices of input.
///
/// # Examples
///
/// Shape some text using the `calt` (Contextual Alternatives) feature.
///
/// ```
/// use harfbuzz_rs::{Face, Font, UnicodeBuffer, shape, Feature, Tag};
///
/// let path = "testfiles/SourceSansVariable-Roman.ttf";
/// let face = Face::from_file(path, 0).expect("could not load face");
/// let font = Font::new(face);
///
/// let buffer = UnicodeBuffer::new().add_str("Hello World!");
///
/// // contextual alternatives feature
/// let feature_tag = b"calt";
///
/// // use the feature on the entire input
/// let feature_range = 0..;
/// let feature = Feature::new(feature_tag, 0, feature_range);
///
/// let output = shape(&font, buffer, &[feature]);
/// ```
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Feature(hb_feature_t);

impl Feature {
    /// Create a new `Feature` struct.
    ///
    /// The feature will be applied with the given value to all glyphs which are
    /// in clusters contained in `range`.
    ///
    /// # Arguments
    ///
    /// - `tag`: The OpenType feature tag to use.
    /// - `value`: Some OpenType features accept different values to change
    ///   their behaviour.
    /// - `range`: The cluster range that should be affected by this feature.
    pub fn new(tag: impl Into<Tag>, value: u32, range: impl RangeBounds<usize>) -> Feature {
        let (start, end) = start_end_range(range);
        Feature(hb_feature_t {
            tag: tag.into().0,
            value,
            start,
            end,
        })
    }

    pub fn tag(&self) -> Tag {
        Tag(self.0.tag)
    }

    pub fn value(&self) -> u32 {
        self.0.value
    }

    pub fn start(&self) -> usize {
        self.0.start as usize
    }

    pub fn end(&self) -> usize {
        self.0.end as usize
    }
}

/// Shape the contents of the buffer using the provided font and activating all
/// OpenType features given in `features`.
///
/// This function consumes the `buffer` and returns a `GlyphBuffer` containing
/// the resulting glyph indices and the corresponding positioning information.
/// Once all the information from the `GlyphBuffer` has been processed as
/// necessary you can reuse the `GlyphBuffer` as an `UnicodeBuffer` (using
/// `GlyphBuffer::clear_contents`) and use that to call `shape` again with new
/// data.
///
/// By default some basic OpenType features are enabled according to the
/// language and the script set in the buffer.
///
/// # Arguments
/// - `font` – a reference to the harfbuzz font used to shape the text.
/// - `buffer` – a `UnicodeBuffer` that is filled with the text to be shaped and
/// also contains metadata about the text in the form of segment properties.
/// - `features` – a slice of additional features to activate
pub fn shape(font: &Font<'_>, buffer: UnicodeBuffer, features: &[Feature]) -> GlyphBuffer {
    let buffer = buffer.guess_segment_properties();
    unsafe {
        hb_shape(
            font.as_raw(),
            buffer.0.as_raw(),
            features.as_ptr() as *mut _,
            features.len() as u32,
        )
    };
    GlyphBuffer(buffer.0)
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, size_of};

    pub(crate) fn assert_memory_layout_equal<T, U>() {
        assert_eq!(size_of::<T>(), size_of::<U>());
        assert_eq!(align_of::<T>(), align_of::<U>());
    }

    #[test]
    fn it_works() {}

    fn assert_feature(feat: Feature, tag: Tag, value: u32, start: usize, end: usize) {
        assert_eq!(feat.tag(), tag);
        assert_eq!(feat.value(), value);
        assert_eq!(feat.start(), start);
        assert_eq!(feat.end(), end);
    }

    use super::{Feature, Tag};
    #[test]
    fn feature_new() {
        let tag = b"abcd".into();
        const UINT_MAX: usize = std::os::raw::c_uint::max_value() as usize;

        let feature = Feature::new(tag, 100, 2..100);
        assert_feature(feature, tag, 100, 2, 100);

        let feature = Feature::new(tag, 100, 2..=100);
        assert_feature(feature, tag, 100, 2, 101);

        let feature = Feature::new(tag, 100, 2..);
        assert_feature(feature, tag, 100, 2, UINT_MAX);

        let feature = Feature::new(tag, 100, ..100);
        assert_feature(feature, tag, 100, 0, 100);

        let feature = Feature::new(tag, 100, ..=100);
        assert_feature(feature, tag, 100, 0, 101);

        let feature = Feature::new(tag, 100, ..);
        assert_feature(feature, tag, 100, 0, UINT_MAX);
    }
}
