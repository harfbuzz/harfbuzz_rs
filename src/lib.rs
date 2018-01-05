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
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! harfbuzz_rs = "^0.1"
//! ```
//!
//! To shape a simple string of text you just create a `Font` from a font file, fill a `Buffer`
//! with some text and call the `shape` function.
//!
//! ```
//! use harfbuzz_rs::*;
//!
//! let path = "path/to/some/font_file.otf";
//! let index = 0;
//! # let path = "testfiles/SourceSansVariable-Roman.ttf";
//! let face = Face::from_file(path, index).unwrap();
//! let mut font = Font::new(face);
//!
//! let output = UnicodeBuffer::new().add_str("Hello World!").shape(&font, &[]);
//! ```
//!
//! The results of the shaping operation are stored in the buffer that was also used as input to
//! the `shape` function.
//!
//! ```
//! # use harfbuzz_rs::*;
//! #
//! # let index = 0;
//! # let path = "testfiles/SourceSansVariable-Roman.ttf";
//! # let face = Face::from_file(path, index).unwrap();
//! # let mut font = Font::new(face);
//! #
//! # let output = UnicodeBuffer::new().add_str("Hello World!").shape(&font, &[]);
//! let positions = output.get_glyph_positions();
//! let infos = output.get_glyph_infos();
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
extern crate harfbuzz_sys as hb;
extern crate libc;

#[macro_use]
extern crate lazy_static;

mod font;
mod font_funcs;
mod blob;
mod face;
pub mod rusttype;
mod common;
mod buffer;

pub use font::*;
pub use face::*;
pub use blob::*;
pub use buffer::*;
pub use common::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
