use hb;
use std::os::raw::c_void;

use std;
use std::marker::PhantomData;

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::ptr::NonNull;

use common::{HarfbuzzObject, Owned, Shared};

/// A `Blob` manages raw data like e.g. file contents. It refers to a slice of
/// bytes that can be either owned by the `Blob` or not.
///
/// Since it is a simple wrapper for `hb_blob_t` it can be used to construct
/// `Font`s and `Face`s directly from memory.
///
/// To enable shared usage of its data it uses a reference counting mechanism
/// making the `clone` operation very cheap as no data is cloned.
///
/// A `Blob` implements `Into` for every type that satisfies the `AsRef<[u8]>'
/// trait such as `Vec<u8>` and `Box<[u8]>` so owned blobs can be created easily
/// from standard Rust objects.
#[repr(C)]
pub struct Blob<'a> {
    _raw: NonNull<hb::hb_blob_t>,
    _marker: PhantomData<&'a mut [u8]>,
}
impl<'a> Blob<'a> {
    /// Create a new `Blob` from the slice `bytes`. The blob will not own the
    /// slice's data.
    pub fn with_bytes(bytes: &'a [u8]) -> Owned<Blob<'a>> {
        let hb_blob = unsafe {
            hb::hb_blob_create(
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                hb::HB_MEMORY_MODE_READONLY,
                0 as *mut _,
                None,
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
        let mut file = File::open(path)?;
        let mut vec = Vec::new();
        file.read_to_end(&mut vec)?;
        Ok(vec.into())
    }

    /// Create a new mutable `Blob` from a slice of bytes.
    pub fn with_mut_bytes(bytes: &mut [u8]) -> Owned<Blob> {
        let hb_blob = unsafe {
            hb::hb_blob_create(
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                hb::HB_MEMORY_MODE_WRITABLE,
                0 as *mut _,
                None,
            )
        };
        unsafe { Owned::from_raw(hb_blob) }
    }

    /// Get a slice of the `Blob`'s bytes.
    pub fn get_data(&self) -> &[u8] {
        unsafe {
            let mut length = hb::hb_blob_get_length(self.as_raw());
            let data_ptr = hb::hb_blob_get_data(self.as_raw(), &mut length as *mut _);
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
        let blob =
            unsafe { hb::hb_blob_create_sub_blob(self.as_raw(), offset as u32, length as u32) };
        unsafe { Shared::from_raw_owned(blob) }
    }

    /// Returns true if the blob is immutable.
    pub fn is_immutable(&self) -> bool {
        unsafe { hb::hb_blob_is_immutable(self.as_raw()) == 1 }
    }

    /// Makes this blob immutable so the bytes it refers to will never change
    /// during its lifetime.
    pub fn make_immutable(&mut self) {
        unsafe { hb::hb_blob_make_immutable(self.as_raw()) }
    }

    /// Try to get a mutable slice of the `Blob`'s bytes, possibly copying them.
    ///
    /// This returns `None` if the blob is immutable or memory allocation
    /// failed.
    pub fn try_get_mut_data(&mut self) -> Option<&'a mut [u8]> {
        unsafe {
            let mut length = hb::hb_blob_get_length(self.as_raw());
            let data_ptr = hb::hb_blob_get_data_writable(self.as_raw(), &mut length as *mut _);
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

impl<'a> fmt::Debug for Blob<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Blob")
            .field("data", &self.get_data())
            .finish()
    }
}

unsafe impl<'a> HarfbuzzObject for Blob<'a> {
    type Raw = hb::hb_blob_t;

    unsafe fn reference(&self) {
        hb::hb_blob_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb::hb_blob_destroy(self.as_raw());
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
        let len = container.as_ref().len();
        let ptr = container.as_ref().as_ptr();

        let data = Box::into_raw(Box::new(container));

        extern "C" fn destroy<U>(ptr: *mut c_void) {
            unsafe { Box::from_raw(ptr as *mut U) };
        }

        let hb_blob = unsafe {
            hb::hb_blob_create(
                ptr as *const i8,
                len as u32,
                hb::HB_MEMORY_MODE_READONLY,
                data as *mut _,
                Some(destroy::<T>),
            )
        };
        unsafe { Shared::from_raw_owned(hb_blob) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_to_blob_conversion() {
        let a_vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let blob: Shared<Blob> = a_vec.into();

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
}
