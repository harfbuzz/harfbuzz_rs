use hb;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
/// A type to represent an opentype feature tag
pub struct Tag(pub hb::hb_tag_t);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TagFromStrErr {
    WrongLength,
    NonAscii
}

use std;
use std::str::FromStr;
use std::ascii::AsciiExt;

impl FromStr for Tag {
    type Err = TagFromStrErr;
    fn from_str(s: &str) -> Result<Tag, TagFromStrErr> {
        if s.is_ascii() == false {
            return Err(TagFromStrErr::NonAscii);
        }
        if s.len() != 4 {
            return Err(TagFromStrErr::WrongLength);
        }
        unsafe {
            Ok(Tag(hb::hb_tag_from_string(s.as_ptr() as *mut _, 4)))
        }
    }
}

use std::fmt::{Debug, Formatter, Error};

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error>  {
        let Tag(ref tag) = *self;
        let mut bytes: [u8; 4] = [0, 0, 0, 0];
        unsafe { hb::hb_tag_to_string(*tag, bytes.as_mut_ptr() as *mut _) };
        let string = std::str::from_utf8(&bytes).expect("UTF8 error while decoding opentype tag.");
        f.write_str(string)
    }
}

/// All trait all wrappers for harfbuzz objects implement. It exposes common functionality for
/// converting from and to the underlying raw harfbuzz pointers.
pub trait HarfbuzzObject: Clone {
    /// Type of the raw harfbuzz object pointer;
    type Raw;

    /// Creates a value safely wrapping the raw harfbuzz pointer. Transfers ownership. _Use of the
    /// original pointer is now forbidden!_ Unsafe because a dereference of a raw pointer is
    /// necesarry.
    unsafe fn from_raw(val: Self::Raw) -> Self;

    /// Creates a value safely wrapping the raw harfbuzz pointer and references it immediately so
    /// that the existing pointer can still be used as normal. Unsafe because a dereference of a
    /// raw pointer is necesarry.
    unsafe fn from_raw_referenced(val: Self::Raw) -> Self {
        let result = Self::from_raw(val);
        std::mem::forget(result.clone()); // increase reference count
        result
    }

    /// Returns the underlying harfbuzz object pointer. The caller must ensure, that this pointer is
    /// not used after the `self`'s destruction.
    fn as_raw(&self) -> Self::Raw;

    /// Returns the underlying harfbuzz object pointer after referencing the object. The resulting
    /// pointer has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the
    /// wrapper using the `from_raw` function.
    fn as_raw_referenced(&self) -> Self::Raw {
        std::mem::forget(self.clone()); // increase reference count
        self.as_raw()
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The resulting pointer
    /// has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the wrapper
    /// using the `from_raw` function.
    fn into_raw(self) -> Self::Raw {
        let result = self.as_raw();
        std::mem::forget(self);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_tag_debugging() {
        let tag = Tag::from_str("ABCD").unwrap();
        assert_eq!("ABCD", format!("{:?}", tag));
    }

    #[test]
    fn test_tag_creation() {
        assert!(Tag::from_str("ABCDE").is_err());
        assert!(Tag::from_str("∞BCD").is_err());
        assert!(Tag::from_str("abWd").is_ok());
    }
}
