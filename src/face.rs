use std;
use hb;
use std::marker::PhantomData;

use blob::Blob;
use font::Font;
use common::{Tag, HarfbuzzObject};

/// A wrapper around the harfbuzz `hb_face_t`.
pub struct Face<'a> {
    hb_face: *mut hb::hb_face_t,
    _marker: PhantomData<&'a [u8]>,
}

use std::path::Path;

impl<'a> Face<'a> {
    /// Create a new `Face` from the data in `bytes`.
    pub fn new<'b, T: Into<Blob<'b>>>(bytes: T, index: u32) -> Face<'b> {
        let blob = bytes.into();
        let hb_face = unsafe { hb::hb_face_create(blob.into_raw(), index) };
        Face {
            hb_face: hb_face,
            _marker: PhantomData,
        }
    }

    /// Create a new face from the contents of the file at `path`.
    pub fn from_file<P: AsRef<Path>>(path: P, index: u32) -> std::io::Result<Face<'static>> {
        let blob = Blob::from_file(path)?;
        Ok(Face::new(blob, index))
    }

    /// Create a `Font` of this face. By default this will use harfbuzz' included opentype font
    /// funcs for shaping and have no scale value set so that the returned values will be in font
    /// space.
    pub fn create_font(&self) -> Font<'a> {
        unsafe {
            let raw_font = hb::hb_font_create(self.hb_face);
            hb::hb_ot_font_set_funcs(raw_font);
            Font::from_raw(raw_font)
        }
    }

    pub fn reference_blob(&self) -> Blob<'a> {
        unsafe {
            let raw_blob = hb::hb_face_reference_blob(self.hb_face);
            Blob::from_raw(raw_blob)
        }
    }

    pub fn index(&self) -> u32 {
        unsafe { hb::hb_face_get_index(self.hb_face) }
    }

    pub fn set_upem(&mut self, upem: u32) {
        unsafe { hb::hb_face_set_upem(self.hb_face, upem) };
    }

    pub fn upem(&self) -> u32 {
        unsafe { hb::hb_face_get_upem(self.hb_face) }
    }

    pub fn set_glyph_count(&mut self, count: u32) {
        unsafe { hb::hb_face_set_glyph_count(self.hb_face, count) };
    }

    pub fn glyph_count(&self) -> u32 {
        unsafe { hb::hb_face_get_glyph_count(self.hb_face) }
    }
}

impl<'a> HarfbuzzObject for Face<'a> {
    type Raw = *mut hb::hb_face_t;

    unsafe fn from_raw(raw: *mut hb::hb_face_t) -> Self {
        Face {
            hb_face: raw,
            _marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut hb::hb_face_t {
        self.hb_face
    }
}

impl<'a> FontTableAccess for Face<'a> {
    fn table_with_tag(&self, tag: Tag) -> Option<Blob> {
        unsafe {
            let raw_blob = hb::hb_face_reference_table(self.hb_face, tag.0);
            if raw_blob.is_null() {
                None
            } else {
                let blob = Blob::from_raw(raw_blob);
                if blob.is_empty() {
                    None
                } else {
                    Some(blob)
                }
            }
        }
    }
}


impl<'a> Clone for Face<'a> {
    fn clone(&self) -> Self {
        let hb_face = unsafe { hb::hb_face_reference(self.hb_face) };
        Face {
            hb_face: hb_face,
            _marker: PhantomData,
        }
    }
}

impl<'a> Drop for Face<'a> {
    fn drop(&mut self) {
        unsafe { hb::hb_face_destroy(self.hb_face) }
    }
}

pub trait FontTableAccess {
    fn table_with_tag(&self, tag: Tag) -> Option<Blob>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use common::Tag;

    #[test]
    fn test_face_wrapper() {
        let font_bytes = include_bytes!("../testfiles/MinionPro-Regular.otf");
        let face = Face::new(&font_bytes[..], 0);
        let blob = face.reference_blob();
        let maxp_table = face.table_with_tag(Tag::from_str("maxp").unwrap()).unwrap();

        assert_eq!(&maxp_table as &[u8], [0x00, 0x00, 0x50, 0x00, 0x06, 0x96]);
        assert_eq!(&blob as &[u8], &font_bytes[..]);
        assert_eq!(face.upem(), 1000);
        assert_eq!(face.glyph_count(), 1686);
    }
}
