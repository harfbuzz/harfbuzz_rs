use std;
use hb;
use libc::c_void;

use std::marker::PhantomData;

use blob::Blob;
use common::{HarfbuzzObject, HbArc, HbBox, Tag};

/// A wrapper around `hb_face_t`.
#[derive(Debug)]
pub struct Face<'a> {
    hb_face: *mut hb::hb_face_t,
    _marker: PhantomData<&'a [u8]>,
}

use std::path::Path;

impl<'a> Face<'a> {
    /// Create a new `Face` from the data in `bytes`.
    pub fn new<'b, T: Into<HbArc<Blob<'b>>>>(bytes: T, index: u32) -> HbBox<Face<'b>> {
        let blob = bytes.into();
        let hb_face = unsafe { hb::hb_face_create(blob.into_raw(), index) };
        unsafe { HbBox::from_raw(hb_face) }
    }

    /// Create a new face from the contents of the file at `path`.
    pub fn from_file<P: AsRef<Path>>(path: P, index: u32) -> std::io::Result<HbBox<Face<'static>>> {
        let blob = Blob::from_file(path)?;
        Ok(Face::new(blob, index))
    }

    /// Create a new face from a closure that returns a raw [`Blob`](struct.Blob.html) of table
    pub fn from_table_func<'b, F>(func: F) -> HbBox<Face<'b>>
    where
        F: FnMut(Tag) -> Option<HbArc<Blob<'b>>>,
    {
        extern "C" fn destroy_box<U>(ptr: *mut c_void) {
            unsafe { Box::from_raw(ptr as *mut U) };
        }
        extern "C" fn table_func<'b, F>(
            _: *mut hb::hb_face_t,
            tag: hb::hb_tag_t,
            user_data: *mut c_void,
        ) -> *mut hb::hb_blob_t
        where
            F: FnMut(Tag) -> Option<HbArc<Blob<'b>>>,
        {
            let tag = Tag(tag);
            let closure = unsafe { &mut *(user_data as *mut F) };
            let blob = closure(tag);
            match blob {
                Some(blob) => blob.into_raw(),
                None => std::ptr::null_mut(),
            }
        }
        let boxed_closure = Box::new(func);
        unsafe {
            let face = hb::hb_face_create_for_tables(
                Some(table_func::<'b, F>),
                Box::into_raw(boxed_closure) as *mut _,
                Some(destroy_box::<F>),
            );
            HbBox::from_raw(face)
        }
    }

    //    /// Create a `Font` of this face. By default this will use harfbuzz' included opentype
    // font
    //    /// funcs for shaping and have no scale value set so that the returned values will be in
    // font
    //    /// space.
    //    pub fn create_font(self) -> HbBox<Font<'a>> {
    //        Font::new(self)
    //    }

    pub fn face_data(&self) -> &'a [u8] {
        unsafe {
            let raw_blob = hb::hb_face_reference_blob(self.hb_face);
            Blob::from_raw(raw_blob).get_data()
        }
    }

    pub fn table_with_tag(&self, tag: Tag) -> Option<&[u8]> {
        unsafe {
            let raw_blob = hb::hb_face_reference_table(self.hb_face, tag.0);
            if raw_blob.is_null() {
                None
            } else {
                let blob = Blob::from_raw(raw_blob);
                if blob.is_empty() {
                    None
                } else {
                    Some(blob.get_data())
                }
            }
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

    unsafe fn reference(&self) -> Self {
        let hb_face = hb::hb_face_reference(self.hb_face);
        Face {
            hb_face: hb_face,
            _marker: PhantomData,
        }
    }

    unsafe fn dereference(&self) {
        hb::hb_face_destroy(self.hb_face);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Tag;

    #[test]
    fn test_face_from_table_func() {
        let face = Face::from_table_func(|table_tag| {
            let content = format!("{}-table", table_tag);
            Some(content.into_bytes().into())
        });

        let maxp_table = face.table_with_tag(Tag::new('m', 'a', 'x', 'p')).unwrap();
        assert_eq!(maxp_table, b"maxp-table");

        let maxp_table = face.table_with_tag(Tag::new('h', 'h', 'e', 'a')).unwrap();
        assert_eq!(maxp_table, b"hhea-table");
    }
}
