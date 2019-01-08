use crate::hb;
use std;
use std::os::raw::c_void;
use std::ptr::NonNull;

use std::marker::PhantomData;
use std::path::Path;

use crate::blob::Blob;
use crate::common::{HarfbuzzObject, Owned, Shared, Tag};

/// A wrapper around `hb_face_t`.
#[derive(Debug)]
pub struct Face<'a> {
    raw: NonNull<hb::hb_face_t>,
    marker: PhantomData<&'a [u8]>,
}

impl<'a> Face<'a> {
    /// Create a new `Face` from the data.
    ///
    /// If `data` is not a valid font then this function returns an empty proxy
    /// value.
    pub fn new<'b, T: Into<Shared<Blob<'b>>>>(data: T, index: u32) -> Owned<Face<'b>> {
        let blob = data.into();
        let hb_face = unsafe { hb::hb_face_create(Shared::into_raw(blob), index) };
        unsafe { Owned::from_raw(hb_face) }
    }

    /// Create a new face from the contents of the file at `path`.
    ///
    /// This function reads the contents of the file at `path` into memory,
    /// creates a `Blob` and then calls `Face::new`.
    ///
    /// See also the discussion in `Blob::from_file`.
    pub fn from_file<P: AsRef<Path>>(path: P, index: u32) -> std::io::Result<Owned<Face<'static>>> {
        let blob = Blob::from_file(path)?;
        Ok(Face::new(blob, index))
    }

    /// Create a face from the bytes of a given slice and an index specifying
    /// which font to read from an OpenType font collection.
    pub fn from_bytes<'b>(bytes: &'b [u8], index: u32) -> Owned<Face<'b>> {
        let blob = Blob::with_bytes(bytes);
        Face::new(blob, index)
    }

    /// Create a new face from a closure that returns a raw
    /// [`Blob`](struct.Blob.html) of table data.
    pub fn from_table_func<'b, F>(func: F) -> Owned<Face<'b>>
    where
        F: 'b + Send + Sync + FnMut(Tag) -> Option<Shared<Blob<'b>>>,
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
            F: FnMut(Tag) -> Option<Shared<Blob<'b>>>,
        {
            let tag = Tag(tag);
            let closure = unsafe { &mut *(user_data as *mut F) };
            let blob = closure(tag);
            match blob {
                Some(blob) => Shared::into_raw(blob),
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
            Owned::from_raw(face)
        }
    }

    pub fn face_data(&self) -> Shared<Blob<'a>> {
        unsafe {
            let raw_blob = hb::hb_face_reference_blob(self.as_raw());
            Shared::from_raw_owned(raw_blob)
        }
    }

    /// Returns the slice of bytes for the table named `tag` or None if there is
    /// no table with `tag`.
    pub fn table_with_tag(&self, tag: Tag) -> Option<Shared<Blob<'a>>> {
        unsafe {
            let raw_blob = hb::hb_face_reference_table(self.as_raw(), tag.0);
            if raw_blob.is_null() {
                None
            } else {
                let blob: Shared<Blob> = Shared::from_raw_owned(raw_blob);
                if blob.is_empty() {
                    None
                } else {
                    Some(blob)
                }
            }
        }
    }

    pub fn index(&self) -> u32 {
        unsafe { hb::hb_face_get_index(self.as_raw()) }
    }

    pub fn set_upem(&mut self, upem: u32) {
        unsafe { hb::hb_face_set_upem(self.as_raw(), upem) };
    }

    pub fn upem(&self) -> u32 {
        unsafe { hb::hb_face_get_upem(self.as_raw()) }
    }

    pub fn set_glyph_count(&mut self, count: u32) {
        unsafe { hb::hb_face_set_glyph_count(self.as_raw(), count) };
    }

    /// Returns the number of glyphs contained in the face.
    pub fn glyph_count(&self) -> u32 {
        unsafe { hb::hb_face_get_glyph_count(self.as_raw()) }
    }
}

unsafe impl<'a> HarfbuzzObject for Face<'a> {
    type Raw = hb::hb_face_t;

    unsafe fn from_raw(raw: *const hb::hb_face_t) -> Self {
        Face {
            raw: NonNull::new_unchecked(raw as *mut _),
            marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut Self::Raw {
        self.raw.as_ptr()
    }

    unsafe fn reference(&self) {
        hb::hb_face_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb::hb_face_destroy(self.as_raw());
    }
}

unsafe impl<'a> Send for Face<'a> {}
unsafe impl<'a> Sync for Face<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Tag;

    #[test]
    fn test_face_from_table_func() {
        let face = Face::from_table_func(|table_tag| {
            let content = format!("{}-table", table_tag);
            Some(content.into_bytes().into())
        });

        let maxp_table = face.table_with_tag(Tag::new('m', 'a', 'x', 'p')).unwrap();
        assert_eq!(maxp_table.as_ref(), b"maxp-table");

        let maxp_table = face.table_with_tag(Tag::new('h', 'h', 'e', 'a')).unwrap();
        assert_eq!(&maxp_table.as_ref(), b"hhea-table");
    }
}
