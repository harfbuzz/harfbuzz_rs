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
//! # extern crate harfbuzz_rs;
//! use harfbuzz_rs::*;
//! use harfbuzz_rs::rusttype::SetRustTypeFuncs;
//!
//! # fn try_main() -> Result<(), std::io::Error> {
//!
//! let path = "path/to/some/font_file.otf";
//! let index = 0; //< face index in the font file
//! # let path = "testfiles/SourceSansVariable-Roman.ttf";
//! let face = Face::from_file(path, index)?;
//! let mut font = Font::new(face);
//! // Use RustType as provider for font information that harfbuzz needs.
//! // You can also use a custom font implementation. For more information look
//! // at the documentation for `FontFuncs`.
//! font.set_rusttype_funcs()?;
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
//! # fn main() {
//! #   try_main().unwrap();
//! # }
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

extern crate harfbuzz_sys as hb;

mod blob;
mod buffer;
mod common;
mod face;
mod font;
mod font_funcs;

#[cfg(feature = "rusttype")]
pub mod rusttype;

#[cfg(feature = "font_parse")]
extern crate font_parse;
#[cfg(feature = "font_parse")]
pub mod rust_font;

pub use blob::*;
pub use buffer::*;
pub use common::*;
pub use face::*;
pub use font::*;

/// Shape the contents of the buffer using the provided font and activating all OpenType features
/// given in `features`.
///
/// This function consumes the `buffer` and returns a `GlyphBuffer` containing the
/// resulting glyph indices and the corresponding positioning information.
pub fn shape(font: &Font, buffer: UnicodeBuffer, features: &[Feature]) -> GlyphBuffer {
    let buffer = buffer.guess_segment_properties();
    unsafe {
        hb::hb_shape(
            font.as_raw(),
            buffer.0.as_raw(),
            features.as_ptr(),
            features.len() as u32,
        )
    };
    GlyphBuffer(buffer.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
