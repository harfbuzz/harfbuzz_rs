use std::os::raw::c_void;
use std::ptr::NonNull;

use std::marker::PhantomData;
use std::path::Path;

use crate::bindings::{
    hb_blob_t, hb_face_create, hb_face_create_for_tables, hb_face_destroy, hb_face_get_empty,
    hb_face_get_glyph_count, hb_face_get_index, hb_face_get_upem, hb_face_reference,
    hb_face_reference_blob, hb_face_reference_table, hb_face_set_glyph_count, hb_face_set_upem,
    hb_face_t, hb_tag_t,
};
use crate::blob::Blob;
use crate::common::{HarfbuzzObject, Owned, Shared, Tag};

/// A wrapper around `hb_face_t`.
///
/// An excerpt from harfbuzz documentation:
/// > Font face is objects represent a single face in a font family. More
/// > exactly, a font face represents a single face in a binary font file. Font
/// > faces are typically built from a binary blob and a face index. Font faces
/// > are used to create fonts.
#[derive(Debug)]
pub struct Face<'a> {
    raw: NonNull<hb_face_t>,
    marker: PhantomData<&'a [u8]>,
}

impl<'a> Face<'a> {
    /// Create a new `Face` from the data.
    ///
    /// If `data` is not a valid font then this function returns the empty face.
    pub fn new<T: Into<Shared<Blob<'a>>>>(data: T, index: u32) -> Owned<Face<'a>> {
        let blob = data.into();
        let hb_face = unsafe { hb_face_create(blob.as_raw(), index) };
        unsafe { Owned::from_raw(hb_face) }
    }

    /// Returns a "null" face.
    pub fn empty() -> Owned<Face<'static>> {
        let hb_face = unsafe { hb_face_get_empty() };
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
    pub fn from_bytes(bytes: &[u8], index: u32) -> Owned<Face<'_>> {
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
            _ = unsafe { Box::from_raw(ptr as *mut U) };
        }
        extern "C" fn table_func<'b, F>(
            _: *mut hb_face_t,
            tag: hb_tag_t,
            user_data: *mut c_void,
        ) -> *mut hb_blob_t
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
            let face = hb_face_create_for_tables(
                Some(table_func::<'b, F>),
                Box::into_raw(boxed_closure) as *mut _,
                Some(destroy_box::<F>),
            );
            Owned::from_raw(face)
        }
    }

    pub fn face_data(&self) -> Shared<Blob<'a>> {
        unsafe {
            let raw_blob = hb_face_reference_blob(self.as_raw());
            Shared::from_raw_owned(raw_blob)
        }
    }

    /// Returns the slice of bytes for the table named `tag` or None if there is
    /// no table with `tag`.
    pub fn table_with_tag(&self, tag: impl Into<Tag>) -> Option<Shared<Blob<'a>>> {
        unsafe {
            let raw_blob = hb_face_reference_table(self.as_raw(), tag.into().0);
            if raw_blob.is_null() {
                None
            } else {
                let blob: Shared<Blob<'_>> = Shared::from_raw_owned(raw_blob);
                if blob.is_empty() {
                    None
                } else {
                    Some(blob)
                }
            }
        }
    }

    pub fn index(&self) -> u32 {
        unsafe { hb_face_get_index(self.as_raw()) }
    }

    pub fn set_upem(&mut self, upem: u32) {
        unsafe { hb_face_set_upem(self.as_raw(), upem) };
    }

    pub fn upem(&self) -> u32 {
        unsafe { hb_face_get_upem(self.as_raw()) }
    }

    pub fn set_glyph_count(&mut self, count: u32) {
        unsafe { hb_face_set_glyph_count(self.as_raw(), count) };
    }

    /// Returns the number of glyphs contained in the face.
    pub fn glyph_count(&self) -> u32 {
        unsafe { hb_face_get_glyph_count(self.as_raw()) }
    }

    #[cfg(variation_support)]
    pub fn get_variation_axis_infos(&self) -> Vec<VariationAxisInfo> {
        let mut count = unsafe { hb_ot_var_get_axis_count(self.as_raw()) };
        let mut vector: Vec<VariationAxisInfo> = Vec::with_capacity(count as usize);
        unsafe {
            hb_ot_var_get_axis_infos(self.as_raw(), 0, &mut count, vector.as_mut_ptr() as *mut _)
        };
        unsafe { vector.set_len(count as usize) };
        vector
    }
}

#[cfg(variation_support)]
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct VariationAxisInfo(pub hb_ot_var_axis_info_t);

unsafe impl<'a> HarfbuzzObject for Face<'a> {
    type Raw = hb_face_t;

    unsafe fn from_raw(raw: *const hb_face_t) -> Self {
        Face {
            raw: NonNull::new(raw as *mut _).unwrap(),
            marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut Self::Raw {
        self.raw.as_ptr()
    }

    unsafe fn reference(&self) {
        hb_face_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb_face_destroy(self.as_raw());
    }
}

unsafe impl<'a> Send for Face<'a> {}
unsafe impl<'a> Sync for Face<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_face_from_table_func() {
        let face = Face::from_table_func(|table_tag| {
            let content = format!("{}-table", table_tag);
            Some(content.into_bytes().into())
        });

        let maxp_table = face.table_with_tag(b"maxp").unwrap();
        assert_eq!(maxp_table.as_ref(), b"maxp-table");

        let maxp_table = face.table_with_tag(b"hhea").unwrap();
        assert_eq!(&maxp_table.as_ref(), b"hhea-table");
    }
}
