use hb;

use std;
use std::marker::PhantomData;

use std::path::Path;
use std::fs::File;
use std::io::Read;

/// A `Blob` manages raw data like e.g. file contents. It refers to a slice of bytes that can be
/// either owned by the `Blob` or not.
///
/// To enable shared usage of its data it uses a reference counting mechanism making the `clone`
/// operation very cheap as no data is cloned.
///
/// A `Blob` implements `Into` for every type that satisfies the `AsRef<[u8]>' trait such as
/// `Vec<u8>` and `Box<[u8]>` so owned blobs can be created easily from standard Rust objects.
pub struct Blob<'a> {
    hb_blob: *mut hb::hb_blob_t,
    _marker: PhantomData<&'a mut hb::hb_blob_t>,
}
impl<'a> Blob<'a> {
    /// Create a new `Blob` from the slice `bytes`. The blob will not own the data.
    pub fn with_bytes(bytes: &[u8]) -> Blob {
        let hb_blob = unsafe {
            hb::hb_blob_create(bytes.as_ptr() as *const i8,
                               bytes.len() as u32,
                               hb::HB_MEMORY_MODE_READONLY,
                               0 as *mut _,
                               None)
        };
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }

    /// Create a `Blob` from the contents of the file at `path`. The entire file is read into
    /// memory and the resulting `Blob` owns this data.
    ///
    /// This can be a performance problem if the file is very big. If this turns out to be a
    /// problem consider `mmap`ing the file or splitting it into smaller chunks before creating a
    /// `Blob`.
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Blob<'static>> {
        let mut file = File::open(path)?;
        let mut vec = Vec::new();
        file.read_to_end(&mut vec)?;
        Ok(vec.into())
    }

    /// Create a new mutable `Blob` from a slice of bytes. Use this only if you *really* know what
    /// you're doing.
    pub fn with_mut_bytes(bytes: &mut [u8]) -> Blob {
        let hb_blob = unsafe {
            hb::hb_blob_create(bytes.as_ptr() as *const i8,
                               bytes.len() as u32,
                               hb::HB_MEMORY_MODE_WRITABLE,
                               0 as *mut _,
                               None)
        };
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }

    /// Make a `Blob` from a raw harfbuzz pointer. Transfers ownership. It is illegal to use the
    /// original pointer after this.
    pub unsafe fn from_raw(blob: *mut hb::hb_blob_t) -> Blob<'a> {
        Blob {
            hb_blob: blob,
            _marker: PhantomData,
        }
    }

    /// Get the underlying harfbuzz blob pointer. The caller must ensure, that this pointer is not
    /// used after the `Blob`'s destruction.
    pub fn get_raw(&self) -> *mut hb::hb_blob_t {
        self.hb_blob
    }

    /// Convert the `Blob` into a raw harfbuzz pointer. This references the underlying harfbuzz
    /// blob. The resulting pointer has to be manually destroyed using `hb_blob_destroy` or
    /// be converted back into a `Blob` using the `from_raw` function.
    pub fn get_raw_referenced(&self) -> *mut hb::hb_blob_t {
        unsafe { hb::hb_blob_reference(self.hb_blob) }
    }

    /// Get a slice of the `Blob`'s bytes.
    pub fn get_data(&self) -> &'a [u8] {
        unsafe {
            let mut length = hb::hb_blob_get_length(self.hb_blob);
            let data_ptr = hb::hb_blob_get_data(self.hb_blob, &mut length as *mut _);
            std::slice::from_raw_parts(data_ptr as *const u8, length as usize)
        }
    }

    /// Creates an immutable `Blob` that contains part of the data of the parent `Blob`. The parent
    /// `Blob` will be immutable after this and the sub`Blob` cannot outlive its parent.
    ///
    /// ### Arguments
    /// * `offset`: Byte-offset of sub-blob within parent.
    /// * `length`: Length of the sub-blob.
    pub fn create_sub_blob(&self, offset: usize, length: usize) -> Blob<'a> {
        let blob =
            unsafe { hb::hb_blob_create_sub_blob(self.hb_blob, offset as u32, length as u32) };
        Blob {
            hb_blob: blob,
            _marker: PhantomData,
        }
    }

    /// Returns true if the blob is immutable.
    pub fn is_immutable(&self) -> bool {
        unsafe { hb::hb_blob_is_immutable(self.hb_blob) == 1 }
    }

    /// Makes this blob immutable so the bytes it refers to will never change during its lifetime.
    pub fn make_immutable(&mut self) {
        unsafe { hb::hb_blob_make_immutable(self.hb_blob) }
    }

    /// Try to get a mutable slice of the `Blob`'s bytes, possibly copying them.
    ///
    /// This returns `None` if the blob is immutable or memory allocation failed.
    pub fn try_get_mut_data(&mut self) -> Option<&'a mut [u8]> {
        unsafe {
            let mut length = hb::hb_blob_get_length(self.hb_blob);
            let data_ptr = hb::hb_blob_get_data_writable(self.hb_blob, &mut length as *mut _);
            if data_ptr.is_null() {
                None
            } else {
                Some(std::slice::from_raw_parts_mut(data_ptr as *mut u8, length as usize))
            }
        }
    }
}

impl<'a> Clone for Blob<'a> {
    /// Creates a new reference to the blob's shared data. Does not copy the data inside the blob.
    fn clone(&self) -> Self {
        let hb_blob = self.get_raw_referenced();
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }
}

use std::ops::Deref;
impl<'a> Deref for Blob<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.get_data()
    }
}

// use std::convert::AsRef;
// impl<'a> AsRef<[u8]> for Blob<'a> {
//     fn as_ref(&self) -> &[u8] {
//         self.get_data()
//     }
// }

use std::convert::From;
impl<'a> From<&'a [u8]> for Blob<'a> {
    fn from(slice: &[u8]) -> Blob {
        Blob::with_bytes(slice)
    }
}

impl<T> From<T> for Blob<'static>
    where T: AsRef<[u8]>
{
    default fn from(bytes: T) -> Blob<'static> {
        let len = bytes.as_ref().len();
        let ptr = bytes.as_ref().as_ptr();

        let user_data = Box::into_raw(Box::new(bytes));

        extern "C" fn destroy<U>(ptr: *mut std::os::raw::c_void) {
            unsafe { Box::from_raw(ptr as *mut U) };
        }

        let hb_blob = unsafe {
            hb::hb_blob_create(ptr as *const i8,
                               len as u32,
                               hb::HB_MEMORY_MODE_READONLY,
                               user_data as *mut _,
                               Some(destroy::<T>))
        };
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }
}

impl<'a> Drop for Blob<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_blob_destroy(self.hb_blob);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_to_blob_test() {
        let a_vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let blob: Blob = a_vec.into();

        assert_eq!(blob.len(), 11);

        let mut counter: u8 = 1;
        for num in blob.iter() {
            assert_eq!(*num, counter);
            counter += 1;
        }
    }

    use std::rc::Rc;
    #[test]
    fn rc_to_blob_test() {
        let rc_bytes: Rc<[u8]> = Rc::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let blob: Blob<'static> = rc_bytes.into();

        assert_eq!(blob.len(), 11);

        let mut counter: u8 = 1;
        for num in blob.iter() {
            assert_eq!(*num, counter);
            counter += 1;
        }
    }
}
