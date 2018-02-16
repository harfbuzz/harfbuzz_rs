use hb;
use std::ops::{Deref, DerefMut};
use std::borrow::Borrow;

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
        write!(
            f,
            "Tag({:?}, {:?}, {:?}, {:?})",
            chars.next().unwrap(),
            chars.next().unwrap(),
            chars.next().unwrap(),
            chars.next().unwrap()
        )
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
pub trait HarfbuzzObject: Sized {
    /// Type of the raw harfbuzz object.
    type Raw;

    /// Creates a reference from a harfbuzz object pointer.
    ///
    /// Unsafe because a raw pointer may be accessed. The reference count is not changed. Should not
    /// be called directly by a library user.
    ///
    /// Use the Owned and Shared abstractions instead.
    unsafe fn from_raw<'a>(val: *const Self::Raw) -> &'a Self {
        &*(val as *const Self)
    }

    /// Creates a mutable reference from a harfbuzz object pointer.
    ///
    /// Unsafe because a raw pointer may be accessed. The reference count is not changed. Should not
    /// be called directly by a library user.
    ///
    /// Use the Owned and Shared abstractions instead.
    unsafe fn from_raw_mut<'a>(val: *mut Self::Raw) -> &'a mut Self {
        &mut *(val as *mut Self)
    }

    /// Returns the underlying harfbuzz object pointer.
    ///
    /// The caller must ensure, that this pointer is not used after `self`'s destruction.
    fn as_raw(&self) -> *mut Self::Raw {
        (((self as *const Self) as *mut Self) as *mut Self::Raw)
    }

    /// Increases the reference count of the HarfBuzz object.
    ///
    /// Wraps a `hb_TYPE_reference()` call.
    unsafe fn reference(&self);

    /// Decreases the reference count of the HarfBuzz object and destroys it if the reference count
    /// reaches zero.
    ///
    /// Wraps a `hb_TYPE_destroy()` call.
    unsafe fn dereference(&self);
}

/// A smart pointer that wraps an atomically reference counted HarfBuzz object.
///
/// Usually you don't create a `Shared` yourself, but get it from another function in this crate.
/// You can just use the methods of the wrapped object through its `Deref` implementation.
///
/// A `Shared` is a safe wrapper for reference counted HarfBuzz objects and provides shared immutable
/// access to its inner object. As HarfBuzz' objects are all thread-safe `Shared` implements `Send`
/// and `Sync`.
///
/// Tries to mirror the stdlib `Arc` interface where applicable as HarfBuzz' reference counting has
/// similar semantics.
#[derive(Debug, PartialEq, Eq)]
pub struct Shared<T: HarfbuzzObject> {
    pointer: *mut T::Raw,
}

impl<T: HarfbuzzObject> Shared<T> {
    /// Creates a `Shared` from a raw harfbuzz pointer.
    ///
    /// Transfers ownership. _Use of the original pointer is now forbidden!_ Unsafe because
    /// dereferencing a raw pointer is necessary.
    pub unsafe fn from_raw(raw: *mut T::Raw) -> Self {
        Shared { pointer: raw }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The resulting pointer
    /// has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the wrapper
    /// using the `from_raw` function to avoid leaking memory.
    pub fn into_raw(shared: Shared<T>) -> *mut T::Raw {
        let result = shared.pointer;
        std::mem::forget(shared);
        result
    }

    pub fn from_ref(reference: &T) -> Self {
        unsafe {
            reference.reference();
            Shared::from_raw(reference.as_raw())
        }
    }
}

impl<T: HarfbuzzObject> Clone for Shared<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.reference();
            Self::from_raw(self.pointer)
        }
    }
}

impl<T: HarfbuzzObject> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { T::from_raw(self.pointer) }
    }
}

impl<T: HarfbuzzObject> Borrow<T> for Shared<T> {
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: HarfbuzzObject> From<Owned<T>> for Shared<T> {
    fn from(t: Owned<T>) -> Self {
        let ptr = t.pointer;
        std::mem::forget(t);
        unsafe { Shared::from_raw(ptr) }
    }
}

impl<T: HarfbuzzObject> Drop for Shared<T> {
    fn drop(&mut self) {
        unsafe { self.dereference() }
    }
}

unsafe impl<T: HarfbuzzObject + Sync + Send> Send for Shared<T> {}
unsafe impl<T: HarfbuzzObject + Sync + Send> Sync for Shared<T> {}

/// A smart pointer that wraps a singly owned harfbuzz object.
///
/// A `Owned` is used to wrap freshly created owned HarfBuzz objects. It permits mutable, non-shared
/// access to the enclosed HarfBuzz value so it can be used e.g. to set up a `Font` or `Face` after
/// its creation.
/// 
/// There is no safe way to construct an `Owned` pointer and usually you don't need to create a 
/// `Owned` yourself, but get it from another function in this crate.
/// You can just use the methods of the wrapped object through its `Deref` implementation.
///
/// Interaction with `Shared`
/// -------------------------
/// When you are finished mutating the inner value, you usually want to pass it to other HarfBuzz
/// functions that expect shared access. Thus you need to convert the `Owned` to a `Shared` pointer
/// using `.into()`. Note however that once a value is
/// converted to  a `Shared<T>`, it will not possible to mutate it anymore.
#[derive(Debug, PartialEq, Eq)]
pub struct Owned<T: HarfbuzzObject> {
    pointer: *mut T::Raw,
}

impl<T: HarfbuzzObject> Owned<T> {
    /// Creates a `Owned` safely wrapping a raw harfbuzz pointer.
    ///
    /// This fully transfers ownership. _Use of the original pointer is now forbidden!_ Unsafe
    /// because a dereference of a raw pointer is necessary.
    ///
    /// Use this only to wrap freshly created HarfBuzz object that is not shared!
    pub unsafe fn from_raw(raw: *mut T::Raw) -> Self {
        Owned { pointer: raw }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The resulting pointer
    /// has to be manually destroyed using `hb_TYPE_destroy` or be converted back into the wrapper
    /// using the `from_raw` function to avoid leaking memory.
    pub fn into_raw(owned: Owned<T>) -> *mut T::Raw {
        let result = owned.pointer;
        std::mem::forget(owned);
        result
    }
}

impl<T: HarfbuzzObject> Drop for Owned<T> {
    fn drop(&mut self) {
        unsafe { self.dereference() }
    }
}

impl<T: HarfbuzzObject> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { T::from_raw(self.pointer) }
    }
}

impl<T: HarfbuzzObject> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { T::from_raw_mut(self.pointer) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::rc::Rc;
    use std::cell::Cell;
    use std::mem;

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

    #[derive(Debug)]
    struct ReferenceCounter {
        rc: Cell<isize>,
    }

    impl HarfbuzzObject for ReferenceCounter {
        type Raw = Cell<isize>;

        unsafe fn reference(&self) {
            println!("referencing {:?}", self);
            let rc = self.rc.get();
            self.rc.set(rc + 1);
        }

        unsafe fn dereference(&self) {
            println!("dereferencing {:?}", self);
            let rc = self.rc.get();
            self.rc.set(rc - 1);
        }
    }

    #[test]
    fn reference_counting_shared() {
        // Mimic a C-API that returns a pointer to a reference counted value.
        let object = Rc::new(ReferenceCounter { rc: Cell::new(1) });
        let raw = Rc::into_raw(object.clone()) as *mut _;

        let arc: Shared<ReferenceCounter> = unsafe { Shared::from_raw(raw) };
        assert_eq!(object.rc.get(), 1);
        {
            let arc2 = Shared::clone(&arc);
            assert_eq!(object.rc.get(), 2);
            mem::drop(arc2);
        }
        assert_eq!(object.rc.get(), 1);
        mem::drop(arc);
        assert_eq!(object.rc.get(), 0);

        // don't leak memory
        let _ = unsafe { Rc::from_raw(raw) };
    }
}
