use std::os::raw::c_void;

use std::marker::PhantomData;

use std::fmt;
use std::fs;
use std::path::Path;
use std::ptr::NonNull;

use crate::bindings::hb_blob_create;
use crate::bindings::hb_blob_create_sub_blob;
use crate::bindings::hb_blob_destroy;
use crate::bindings::hb_blob_get_data;
use crate::bindings::hb_blob_get_data_writable;
use crate::bindings::hb_blob_get_length;
use crate::bindings::hb_blob_is_immutable;
use crate::bindings::hb_blob_make_immutable;
use crate::bindings::hb_blob_reference;
use crate::bindings::hb_blob_t;
use crate::bindings::hb_memory_mode_t_HB_MEMORY_MODE_READONLY as HB_MEMORY_MODE_READONLY;
use crate::bindings::hb_memory_mode_t_HB_MEMORY_MODE_WRITABLE as HB_MEMORY_MODE_WRITABLE;
use crate::common::{HarfbuzzObject, Owned, Shared};

/// A `Blob` manages raw data like e.g. file contents. It refers to a slice of
/// bytes that can be either owned by the `Blob` or not.
///
/// It is used to provide access to memory for `Face` and `Font`. Typically it
/// contains the raw font data (e.g. an entire font file or font tables). To enable
/// shared usage of its data it uses a reference counting mechanism making the
/// `clone` operation very cheap as no data is cloned.
///
/// # Construction
///
/// A `Blob` implements `Into` for every type that satisfies the `AsRef<[u8]>`
/// trait such as `Vec<u8>` and `Box<[u8]>` so owned blobs can be created easily
/// from standard Rust objects.
///
/// You can also create `Blob`s that contain borrowed data using the constructors
/// `Blob::with_bytes` and `Blob::with_bytes_mut` for immutable and mutable access
/// respectively.
#[repr(C)]
pub struct Blob<'a> {
    raw: NonNull<hb_blob_t>,
    marker: PhantomData<&'a [u8]>,
}
impl<'a> Blob<'a> {
    /// Create a new `Blob` from the slice `bytes`. The blob will not own the
    /// slice's data.
    pub fn with_bytes(bytes: &'a [u8]) -> Owned<Blob<'a>> {
        let hb_blob = unsafe {
            hb_blob_create(
                bytes.as_ptr() as *const _,
                bytes.len() as u32,
                HB_MEMORY_MODE_READONLY,
                std::ptr::null_mut(),
                None,
            )
        };
        unsafe { Owned::from_raw(hb_blob) }
    }

    /// Create a new `Blob` from the mutable slice `bytes`. The blob will not own the
    /// slice's data.
    pub fn with_bytes_mut(bytes: &'a mut [u8]) -> Owned<Blob<'a>> {
        let hb_blob = unsafe {
            hb_blob_create(
                bytes.as_ptr() as *const _,
                bytes.len() as u32,
                HB_MEMORY_MODE_WRITABLE,
                std::ptr::null_mut(),
                None,
            )
        };
        unsafe { Owned::from_raw(hb_blob) }
    }

    /// Create a new `Blob` from a type that owns a byte slice, effectively handing over
    /// ownership of its data to the blob.
    pub fn with_bytes_owned<T: 'a + Send>(
        bytes_owner: T,
        projector: impl Fn(&T) -> &[u8],
    ) -> Owned<Blob<'a>> {
        let boxxed = Box::new(bytes_owner);
        let slice = projector(&boxxed);
        let len = slice.len();
        let ptr = slice.as_ptr();

        let data = Box::into_raw(boxxed);

        extern "C" fn destroy<U>(ptr: *mut c_void) {
            _ = unsafe { Box::from_raw(ptr as *mut U) };
        }

        let hb_blob = unsafe {
            hb_blob_create(
                ptr as *const _,
                len as u32,
                HB_MEMORY_MODE_READONLY,
                data as *mut _,
                Some(destroy::<T>),
            )
        };
        unsafe { Owned::from_raw(hb_blob) }
    }

    /// Create a `Blob` from the contents of the file at `path` whose contents
    /// will be read into memory.
    ///
    /// The result will be either a `Blob` that owns the file's contents or an
    /// error that happened while trying to read the file.
    ///
    /// This can be a performance problem if the file is very big. If this turns
    /// out to be a problem consider `mmap`ing the file or splitting it into
    /// smaller chunks before creating a `Blob`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Shared<Blob<'static>>> {
        let vec = fs::read(path)?;
        Ok(vec.into())
    }

    /// Get a slice of the `Blob`'s bytes.
    pub fn get_data(&self) -> &[u8] {
        unsafe {
            let mut length = hb_blob_get_length(self.as_raw());
            let data_ptr = hb_blob_get_data(self.as_raw(), &mut length as *mut _);
            std::slice::from_raw_parts(data_ptr as *const u8, length as usize)
        }
    }

    /// Creates an immutable `Blob` that contains part of the data of the parent
    /// `Blob`. The parent `Blob` will be immutable after this and the sub`Blob`
    /// cannot outlive its parent.
    ///
    /// ### Arguments
    /// * `offset`: Byte-offset of sub-blob within parent.
    /// * `length`: Length of the sub-blob.
    pub fn create_sub_blob(&self, offset: usize, length: usize) -> Shared<Blob<'a>> {
        let blob = unsafe { hb_blob_create_sub_blob(self.as_raw(), offset as u32, length as u32) };
        unsafe { Shared::from_raw_owned(blob) }
    }

    /// Returns true if the blob is immutable.
    ///
    /// HarfBuzz internally uses this value to make sure the blob is not mutated
    /// after being shared. In Rust this is not really necessary due to the borrow
    /// checker. This method is provided regardless for completeness.
    pub fn is_immutable(&self) -> bool {
        unsafe { hb_blob_is_immutable(self.as_raw()) == 1 }
    }

    /// Makes this blob immutable so the bytes it refers to will never change
    /// during its lifetime.
    pub fn make_immutable(&mut self) {
        unsafe { hb_blob_make_immutable(self.as_raw()) }
    }

    /// Try to get a mutable slice of the `Blob`'s bytes, possibly copying them.
    ///
    /// This returns `None` if the blob is immutable or memory allocation
    /// failed.
    pub fn try_get_mut_data(&mut self) -> Option<&'a mut [u8]> {
        unsafe {
            let mut length = hb_blob_get_length(self.as_raw());
            let data_ptr = hb_blob_get_data_writable(self.as_raw(), &mut length as *mut _);
            if data_ptr.is_null() {
                None
            } else {
                Some(std::slice::from_raw_parts_mut(
                    data_ptr as *mut u8,
                    length as usize,
                ))
            }
        }
    }
}

unsafe impl<'a> Send for Blob<'a> {}
unsafe impl<'a> Sync for Blob<'a> {}

impl<'a> fmt::Debug for Blob<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blob")
            .field("data", &self.get_data())
            .field("is_immutable", &self.is_immutable())
            .finish()
    }
}

unsafe impl<'a> HarfbuzzObject for Blob<'a> {
    type Raw = hb_blob_t;

    unsafe fn from_raw(raw: *const hb_blob_t) -> Self {
        Blob {
            raw: NonNull::new(raw as *mut _).unwrap(),
            marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut hb_blob_t {
        self.raw.as_ptr()
    }

    unsafe fn reference(&self) {
        hb_blob_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb_blob_destroy(self.as_raw());
    }
}

use std::ops::Deref;
impl<'a> Deref for Blob<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.get_data()
    }
}

use std::convert::AsRef;
impl<'a> AsRef<[u8]> for Blob<'a> {
    fn as_ref(&self) -> &[u8] {
        self.get_data()
    }
}

use std::convert::From;

impl<'a, T> From<T> for Shared<Blob<'a>>
where
    T: 'a + Send + AsRef<[u8]>,
{
    fn from(container: T) -> Shared<Blob<'a>> {
        let blob = Blob::with_bytes_owned(container, |t| t.as_ref());
        blob.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_to_blob_conversion() {
        let a_vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let blob: Shared<Blob<'_>> = a_vec.into();

        assert_eq!(blob.len(), 11);

        let mut counter: u8 = 1;
        for num in blob.iter() {
            assert_eq!(*num, counter);
            counter += 1;
        }
    }

    use std::sync::Arc;
    #[test]
    fn test_arc_to_blob_conversion() {
        let rc_slice: Arc<[u8]> = Arc::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let blob: Shared<Blob<'static>> = rc_slice.into();

        assert_eq!(blob.len(), 11);

        let mut counter: u8 = 1;
        for num in blob.iter() {
            assert_eq!(*num, counter);
            counter += 1;
        }
    }

    #[test]
    fn test_arc_vec_to_blob_conversion() {
        let rc_slice: Arc<Vec<u8>> = Arc::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let blob: Owned<Blob<'static>> = Blob::with_bytes_owned(rc_slice.clone(), |t| &*t);
        assert_eq!(Arc::strong_count(&rc_slice), 2);

        assert_eq!(blob.len(), 11);

        let mut counter: u8 = 1;
        for num in blob.iter() {
            assert_eq!(*num, counter);
            counter += 1;
        }

        std::mem::drop(blob);
        assert_eq!(Arc::strong_count(&rc_slice), 1);
    }
}
