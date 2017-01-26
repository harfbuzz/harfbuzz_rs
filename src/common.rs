use hb;

/// A type to represent 4 byte tags that are used in many font formats for naming font tables,
/// font features and similar.
///
/// The user-facing representation is a 4-character ASCII string. `Tag` provides methods to create
/// `Tag`s from such a representation and to get the string representation from a `Tag`.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct Tag(pub hb::hb_tag_t);

impl Tag {
    /// Create a `Tag` from its four-char textual representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use harfbuzz_rs::Tag;
    /// let cmap_tag = Tag::new('c', 'm', 'a', 'p');
    /// assert_eq!(cmap_tag.to_string(), "cmap")
    /// ```
    ///
    pub fn new(a: char, b: char, c: char, d: char) -> Self {
        Tag(((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32))
    }

    fn tag_to_string(self) -> String {
        let mut buf: [u8; 4] = [0; 4];
        unsafe { hb::hb_tag_to_string(self.0, buf.as_mut_ptr() as *mut _) };
        String::from_utf8_lossy(&buf).into()
    }
}

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let string = self.tag_to_string();
        let mut chars = string.chars().chain(std::iter::repeat('\u{FFFD}'));
        write!(f,
               "Tag({:?}, {:?}, {:?}, {:?})",
               chars.next().unwrap(),
               chars.next().unwrap(),
               chars.next().unwrap(),
               chars.next().unwrap())
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.tag_to_string())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// An Error generated when a `Tag` fails to parse from a `&str` with the `from_str` function.
pub enum TagFromStrErr {
    /// The string contains non-ASCII characters.
    NonAscii,
    /// The string has length zero.
    ZeroLengthString,
}

use std;
use std::str::FromStr;
use std::ascii::AsciiExt;

impl FromStr for Tag {
    type Err = TagFromStrErr;
    /// Parses a `Tag` from a `&str` that contains four or less ASCII characters. When the string's
    /// length is smaller than 4 it is extended with `' '` (Space) characters. The remaining bytes
    /// of strings longer than 4 bytes are ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use harfbuzz_rs::Tag;
    /// use std::str::FromStr;
    /// let tag1 = Tag::from_str("ABCD").unwrap();
    /// let tag2 = Tag::new('A', 'B', 'C', 'D');
    /// assert_eq!(tag1, tag2);
    /// ```
    ///
    fn from_str(s: &str) -> Result<Tag, TagFromStrErr> {
        if s.is_ascii() == false {
            return Err(TagFromStrErr::NonAscii);
        }
        if s.len() == 0 {
            return Err(TagFromStrErr::ZeroLengthString);
        }
        let len = std::cmp::max(s.len(), 4) as i32;
        unsafe { Ok(Tag(hb::hb_tag_from_string(s.as_ptr() as *mut _, len))) }
    }
}

pub struct Language(pub hb::hb_language_t);

impl Default for Language {
    fn default() -> Language {
        Language(unsafe { hb::hb_language_get_default() })
    }
}

impl Debug for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Language(\"{}\")", self)
    }
}

use std::ffi::CStr;
impl Display for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let string = unsafe {
            let char_ptr = hb::hb_language_to_string(self.0);
            CStr::from_ptr(char_ptr)
                .to_str()
                .expect("String representation of language is not valid utf8.")
        };
        write!(f, "{}", string)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidLanguage;

impl FromStr for Language {
    type Err = InvalidLanguage;
    fn from_str(s: &str) -> Result<Language, InvalidLanguage> {
        let len = std::cmp::min(s.len(), std::i32::MAX as _) as i32;
        let lang = unsafe { hb::hb_language_from_string(s.as_ptr() as *mut _, len) };
        if lang.is_null() {
            Err(InvalidLanguage {})
        } else {
            Ok(Language(lang))
        }
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
        assert_eq!("ABCD", format!("{}", tag));
        assert_eq!("Tag('A', 'B', 'C', 'D')", format!("{:?}", tag));
    }

    #[test]
    fn test_tag_creation() {
        assert!(Tag::from_str("∞BCD").is_err());
        assert!(Tag::from_str("").is_err());
        assert_eq!(Tag::from_str("ABCDE"), Tag::from_str("ABCD"));
        assert_eq!(Tag::from_str("abWd").unwrap(), Tag::new('a', 'b', 'W', 'd'));
    }

    #[test]
    fn test_language() {
        assert_eq!(Language::default().to_string(), "c");
        assert_eq!(Language::from_str("ger").unwrap().to_string(), "ger");
        assert_eq!(Language::from_str("ge!").unwrap().to_string(), "ge");
        assert_eq!(Language::from_str("German").unwrap().to_string(), "german");
    }
}
