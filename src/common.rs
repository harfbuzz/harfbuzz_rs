use hb;
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, ToOwned};

/// A type to represent 4-byte SFNT tags.
///
/// Tables, features, etc. in OpenType and many other font formats use SFNT tags as identifiers.
/// These are 4-bytes long and usually each byte represents an ASCII value. `Tag` provides methods
/// to create such identifiers from individual `chars` or a `str` slice and to get the string
/// representation of a `Tag`.
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
            if char_ptr.is_null() {
                return Err(fmt::Error);
            }
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


/// A trait which is implemented for all harffbuzz wrapper structs. It exposes common functionality
/// for converting from and to the underlying raw harfbuzz pointers that are useful for ffi.
pub trait HarfbuzzObject {
    /// Type of the raw harfbuzz object pointer.
    type Raw;

    /// Creates a value from a harfbuzz object pointer.
    ///
    /// Unsafe because a raw pointer may be accessed. The reference count is not changed. Should not
    /// be called directly by a library user.
    unsafe fn from_raw(val: Self::Raw) -> Self;

    /// Returns the underlying harfbuzz object pointer.
    ///
    /// The caller must ensure, that this pointer is not used after `self`'s destruction.
    fn as_raw(&self) -> Self::Raw;

    /// Increases the reference count of the HarfBuzz object.
    ///
    /// Wraps a `hb_TYPE_reference()` call.
    unsafe fn reference(&self) -> Self;

    /// Decreases the reference count of the HarfBuzz object and destroys it if the reference count
    /// reaches zero.
    ///
    /// Wraps a `hb_TYPE_destroy()` call.
    unsafe fn dereference(&self);
}

/// Wraps an atomically reference counted HarfBuzz object.
///
/// A `HbArc` is a safe wrapper for reference counted HarfBuzz objects and provides shared immutable
/// access to its inner object. As HarfBuzz' objects are all thread-safe `HbArc` implements `Send`
/// and `Sync`.
///
/// Tries to mirror the stdlib `Arc` interface where applicable as HarfBuzz' reference counting has
/// similar semantics.
#[derive(Debug, PartialEq, Eq)]
pub struct HbArc<T: HarfbuzzObject> {
    object: HbRef<T>,
}

impl<T: HarfbuzzObject> HbArc<T> {
    /// Creates a `HbArc` from a raw harfbuzz pointer.
    ///
    /// Transfers ownership. _Use of the original pointer is now forbidden!_ Unsafe because a
    /// dereference of a raw pointer is necessary.
    pub unsafe fn from_raw(raw: T::Raw) -> Self {
        HbArc { object: HbRef::from_raw(raw) }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The resulting pointer
    /// has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the wrapper
    /// using the `from_raw` function.
    pub fn into_raw(self) -> T::Raw {
        let result = self.object.as_raw();
        std::mem::forget(self);
        result
    }
}

impl<T: HarfbuzzObject> Clone for HbArc<T> {
    fn clone(&self) -> Self {
        unsafe { HbArc { object: HbRef { object: self.object.reference() } } }
    }
}

impl<T: HarfbuzzObject> Deref for HbArc<T> {
    type Target = HbRef<T>;

    fn deref(&self) -> &HbRef<T> {
        &self.object
    }
}

impl<T: HarfbuzzObject> Borrow<HbRef<T>> for HbArc<T> {
    fn borrow(&self) -> &HbRef<T> {
        &self
    }
}

impl<T: HarfbuzzObject> From<HbBox<T>> for HbArc<T> {
    fn from(t: HbBox<T>) -> Self {
        let raw = t.object.as_raw();
        std::mem::forget(t);
        unsafe { HbArc::from_raw(raw) }
    }
}

impl<T: HarfbuzzObject> From<HbRef<T>> for HbArc<T> {
    fn from(t: HbRef<T>) -> Self {
        HbArc { object: t }
    }
}

impl<T: HarfbuzzObject> Drop for HbArc<T> {
    fn drop(&mut self) {
        unsafe { self.object.dereference() }
    }
}

unsafe impl<T: HarfbuzzObject + Sync + Send> Send for HbArc<T> {}
unsafe impl<T: HarfbuzzObject + Sync + Send> Sync for HbArc<T> {}

/// Wraps a reference to a harfbuzz object and provides immutable access to it through its `Deref`
/// implementation.
///
/// A `HbRef` does not own its content.
#[derive(Debug, PartialEq, Eq)]
pub struct HbRef<T: HarfbuzzObject> {
    object: T,
}

impl<T: HarfbuzzObject> HbRef<T> {
    /// Creates a `HbRef`that safely wraps a reference to a raw harfbuzz object.
    ///
    /// This does not transfer ownership. Special care must be taken that the harfbuzz object is not
    /// destroyed while a `HbRef` is in use.
    ///
    /// Unsafe because a dereference of a raw pointer is necessary.
    pub unsafe fn from_raw(raw: T::Raw) -> Self {
        HbRef { object: T::from_raw(raw) }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value.
    ///
    /// The resulting pointer has to be manually destroyed using `hb_TYPE_destroy` or be converted
    /// back into the wrapper using the `from_raw` function.
   pub fn as_raw(&self) -> T::Raw {
       self.object.as_raw()
   }
}

impl<T: HarfbuzzObject> ToOwned for HbRef<T> {
    type Owned = HbArc<T>;

    fn to_owned(&self) -> Self::Owned {
        HbArc { object: HbRef { object: unsafe { self.reference() } } }
    }
}

impl<T: HarfbuzzObject> Deref for HbRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.object
    }
}

/// Wraps a mutable owned harfbuzz object.
///
/// This is used to wrap freshly created owned HarfBuzz objects. It permits mutable, non-shared
/// access to the enclosed HarfBuzz value so it can be used e.g. to set up a `Font` or `Face` after
/// its creation.
///
/// When you are finished mutating the value, you usually want to pass it to other HarfBuzz
/// functions that expects shared access. This can be accomplished with the `to_arc` method
/// that takes a `HbBox<T>` and turns it into a `HbArc<T>` which uses atomic reference counting to
/// manage thread-safe and shared access to its inner resource. Note however that once a value is
/// converted to  a `HbArc<T>`, it will not possible to mutate it anymore.
#[derive(Debug, PartialEq, Eq)]
pub struct HbBox<T: HarfbuzzObject> {
    object: T,
}

impl<T: HarfbuzzObject> HbBox<T> {
    /// Creates a `HbBox` safely wrapping a raw harfbuzz pointer.
    ///
    /// This fully transfers ownership. _Use of the original pointer is now forbidden!_ Unsafe
    /// because a dereference of a raw pointer is necessary.
    ///
    /// Use this only to wrap freshly created HarfBuzz object!
    pub unsafe fn from_raw(raw: T::Raw) -> Self {
        HbBox { object: T::from_raw(raw) }
    }

    pub fn into_arc(self) -> HbArc<T> {
        self.into()
    }

    // TODO
    //    /// Converts `self` into the underlying harfbuzz object pointer value. The resulting
    // pointer
    //    /// has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the
    // wrapper
    //    /// using the `from_raw` function.
    //    pub fn as_raw(&self) -> T::Raw {
    //        self.object.as_raw()
    //    }
}

impl<T: HarfbuzzObject> Drop for HbBox<T> {
    fn drop(&mut self) {
        unsafe { self.object.dereference() }
    }
}

impl<T: HarfbuzzObject> Deref for HbBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.object
    }
}

impl<T: HarfbuzzObject> DerefMut for HbBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.object
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
