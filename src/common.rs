use crate::hb;
use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};

/// A type to represent 4-byte SFNT tags.
///
/// The easiest way to create a tag is by using its `From<&[u8; 4]>` impl:
///
/// ```
/// # use harfbuzz_rs::Tag;
/// let tag: Tag = b"abcd".into();
/// assert_eq!(&tag.to_bytes(), b"abcd");
/// ```
///
/// Tables, features, etc. in OpenType and many other font formats use SFNT tags
/// as identifiers. These are 4-bytes long and usually each byte represents an
/// ASCII value. `Tag` provides methods to create such identifiers from
/// individual `chars` or a `str` slice and to get the string representation of
/// a `Tag`.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Tag(pub hb::hb_tag_t);

impl Tag {
    /// Create a `Tag` from its four-char textual representation.
    ///
    /// All the arguments must be ASCII values.
    ///
    /// # Examples
    ///
    /// ```
    /// use harfbuzz_rs::Tag;
    /// let cmap_tag = Tag::new('c', 'm', 'a', 'p');
    /// assert_eq!(cmap_tag.to_string(), "cmap")
    /// ```
    ///
    pub const fn new(a: char, b: char, c: char, d: char) -> Self {
        Tag(((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32))
    }

    fn tag_to_string(self) -> String {
        let mut buf: [u8; 4] = [0; 4];
        unsafe { hb::hb_tag_to_string(self.0, buf.as_mut_ptr() as *mut _) };
        String::from_utf8_lossy(&buf).into()
    }

    /// Returns tag as 4-element byte array.
    ///
    /// # Examples
    /// ```
    /// # use harfbuzz_rs::Tag;
    /// let tag = Tag::new('a', 'b', 'c', 'd');
    /// assert_eq!(&tag.to_bytes(), b"abcd");
    /// ```
    pub const fn to_bytes(self) -> [u8; 4] {
        #[allow(clippy::identity_op)]
        [
            (self.0 >> 24 & 0xff) as u8,
            (self.0 >> 16 & 0xff) as u8,
            (self.0 >> 8 & 0xff) as u8,
            (self.0 >> 0 & 0xff) as u8,
        ]
    }
}

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl<'a> From<&'a [u8; 4]> for Tag {
    fn from(byte_array: &'a [u8; 4]) -> Tag {
        Tag::new(
            byte_array[0] as char,
            byte_array[1] as char,
            byte_array[2] as char,
            byte_array[3] as char,
        )
    }
}

impl From<Tag> for [u8; 4] {
    fn from(tag: Tag) -> [u8; 4] {
        tag.to_bytes()
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tag_to_string())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// An Error generated when a `Tag` fails to parse from a `&str` with the
/// `from_str` function.
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
    /// Parses a `Tag` from a `&str` that contains four or less ASCII
    /// characters. When the string's length is smaller than 4 it is extended
    /// with `' '` (Space) characters. The remaining bytes of strings longer
    /// than 4 bytes are ignored.
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
        if !s.is_ascii() {
            return Err(TagFromStrErr::NonAscii);
        }
        if s.is_empty() {
            return Err(TagFromStrErr::ZeroLengthString);
        }
        let len = std::cmp::max(s.len(), 4) as i32;
        unsafe { Ok(Tag(hb::hb_tag_from_string(s.as_ptr() as *mut _, len))) }
    }
}

/// Defines the direction in which text is to be read.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Initial, unset direction.
    Invalid,
    /// Text is set horizontally from left to right.
    Ltr,
    /// Text is set horizontally from right to left.
    Rtl,
    /// Text is set vertically from top to bottom.
    Ttb,
    /// Text is set vertically from bottom to top.
    Btt,
}

impl Direction {
    /// Convert into raw value of type `hb_direction_t`.
    pub fn to_raw(self) -> hb::hb_direction_t {
        match self {
            Direction::Invalid => hb::HB_DIRECTION_INVALID,
            Direction::Ltr => hb::HB_DIRECTION_LTR,
            Direction::Rtl => hb::HB_DIRECTION_RTL,
            Direction::Ttb => hb::HB_DIRECTION_TTB,
            Direction::Btt => hb::HB_DIRECTION_BTT,
        }
    }

    /// Create from raw value of type `hb_direction_t`.
    pub fn from_raw(dir: hb::hb_direction_t) -> Self {
        match dir {
            hb::HB_DIRECTION_LTR => Direction::Ltr,
            hb::HB_DIRECTION_RTL => Direction::Rtl,
            hb::HB_DIRECTION_TTB => Direction::Ttb,
            hb::HB_DIRECTION_BTT => Direction::Btt,
            _ => Direction::Invalid,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Language(pub hb::hb_language_t);

impl Default for Language {
    fn default() -> Language {
        Language(unsafe { hb::hb_language_get_default() })
    }
}

impl Debug for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Language(\"{}\")", self)
    }
}

use std::ffi::CStr;
impl Display for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Script(pub hb::hb_script_t);

impl Script {
    pub fn from_iso15924_tag(tag: Tag) -> Self {
        Script(unsafe { hb::hb_script_from_iso15924_tag(tag.0) })
    }

    pub fn to_iso15924_tag(self) -> Tag {
        Tag(unsafe { hb::hb_script_to_iso15924_tag(self.0) })
    }

    pub fn horizontal_direction(self) -> Direction {
        Direction::from_raw(unsafe { hb::hb_script_get_horizontal_direction(self.0) })
    }
}

/// A trait which is implemented for all harffbuzz wrapper structs. It exposes
/// common functionality for converting from and to the underlying raw harfbuzz
/// pointers that are useful for ffi.
///
/// # Safety
///
/// This trait may only be implemented for structs that are zero-sized and is
/// therefore unsafe to implement.
pub unsafe trait HarfbuzzObject: Sized {
    /// Type of the raw harfbuzz object.
    type Raw;

    /// Creates a reference from a harfbuzz object pointer.
    ///
    /// Unsafe because a raw pointer may be accessed. The reference count is not
    /// changed. Should not be called directly by a library user.
    ///
    /// Use the Owned and Shared abstractions instead.
    #[doc(hidden)]
    unsafe fn from_raw(val: *const Self::Raw) -> Self;

    /// Returns the underlying harfbuzz object pointer.
    ///
    /// The caller must ensure, that this pointer is not used after `self`'s
    /// destruction.
    fn as_raw(&self) -> *mut Self::Raw;

    /// Increases the reference count of the HarfBuzz object.
    ///
    /// Wraps a `hb_TYPE_reference()` call.
    ///
    /// # Safety
    ///
    /// While no undefined behavior can be introduced only by increasing the
    /// reference count (I think) this method is still marked unsafe since there
    /// should be no need for it to be called from safe code.
    unsafe fn reference(&self);

    /// Decreases the reference count of the HarfBuzz object and destroys it if
    /// the reference count reaches zero.
    ///
    /// Wraps a `hb_TYPE_destroy()` call.
    ///
    /// # Safety
    ///
    /// You always have to call `reference` first before using this method.
    /// Otherwise you might accidentally hold on to already destroyed objects
    /// and causing UB.
    unsafe fn dereference(&self);
}

/// A smart pointer that wraps an atomically reference counted HarfBuzz object.
///
/// Usually you don't create a `Shared` yourself, but get it from another
/// function in this crate. You can just use the methods of the wrapped object
/// through its `Deref` implementation.
///
/// A `Shared` is a safe wrapper for reference counted HarfBuzz objects and
/// provides shared immutable access to its inner object. As HarfBuzz' objects
/// are all thread-safe `Shared` implements `Send` and `Sync`.
///
/// Tries to mirror the stdlib `Arc` interface where applicable as HarfBuzz'
/// reference counting has similar semantics.
#[derive(Debug, PartialEq, Eq)]
pub struct Shared<T: HarfbuzzObject> {
    object: T,
}

impl<T: HarfbuzzObject> Shared<T> {
    /// Creates a `Shared` from an owned raw harfbuzz pointer.
    ///
    /// # Safety
    ///
    /// Transfers ownership. _Use of the original pointer is now forbidden!_
    /// Unsafe because dereferencing a raw pointer is necessary.
    pub unsafe fn from_raw_owned(raw: *mut T::Raw) -> Self {
        let object = T::from_raw(raw);
        Shared { object }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The
    /// resulting pointer has to be manually destroyed using `hb_TYPE_destroy`
    /// or be converted back into the wrapper using the `from_raw` function to
    /// avoid leaking memory.
    pub fn into_raw(shared: Shared<T>) -> *mut T::Raw {
        let result = shared.object.as_raw();
        std::mem::forget(shared);
        result
    }

    /// Creates a `Shared` by cloning a raw harfbuzz pointer.

    ///
    /// The original pointer can still be safely used but must be released at
    /// the end to avoid memory leaks.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid pointer to the corresponding harfbuzz object or
    /// the behavior is undefined.
    ///
    /// Internally this method increases the reference count of `raw` so it is
    /// safe to call `hb_destroy_[...]` on `raw` after using `from_raw_ref`.
    pub unsafe fn from_raw_ref(raw: *mut T::Raw) -> Self {
        let object = T::from_raw(raw);
        object.reference();
        Shared { object }
    }
}

impl<T: HarfbuzzObject> Clone for Shared<T> {
    /// Returns a copy and increases the reference count.
    ///
    /// This behaviour is exactly like `Arc::clone` in the standard library.
    fn clone(&self) -> Self {
        unsafe { Self::from_raw_ref(self.object.as_raw()) }
    }
}

impl<T: HarfbuzzObject> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.object
    }
}

impl<T: HarfbuzzObject> Borrow<T> for Shared<T> {
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: HarfbuzzObject> From<Owned<T>> for Shared<T> {
    fn from(t: Owned<T>) -> Self {
        let ptr = t.object.as_raw();
        std::mem::forget(t);
        unsafe { Shared::from_raw_owned(ptr) }
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
/// A `Owned` is used to wrap freshly created owned HarfBuzz objects. It permits
/// mutable, non-shared access to the enclosed HarfBuzz value so it can be used
/// e.g. to set up a `Font` or `Face` after its creation.
///
/// There is no safe way to construct an `Owned` pointer and usually you don't
/// need to create a `Owned` yourself, but get it from another function in this
/// crate. You can just use the methods of the wrapped object through its
/// `Deref` implementation.
///
/// Interaction with `Shared`
/// -------------------------
/// When you are finished mutating the inner value, you usually want to pass it
/// to other HarfBuzz functions that expect shared access. Thus you need to
/// convert the `Owned` to a `Shared` pointer using `.into()`. Note however that
/// once a value is converted to  a `Shared<T>`, it will not possible to mutate
/// it anymore.
#[derive(Debug, PartialEq, Eq)]
pub struct Owned<T: HarfbuzzObject> {
    object: T,
}

impl<T: HarfbuzzObject> Owned<T> {
    /// Creates a `Owned` safely wrapping a raw harfbuzz pointer.
    ///
    /// # Safety
    ///
    /// This fully transfers ownership. _Use of the original pointer is now
    /// forbidden!_ Unsafe because a dereference of a raw pointer is necessary.
    ///
    /// Use this only to wrap freshly created HarfBuzz object that is not
    /// shared! Otherwise it is possible to have aliasing mutable references
    /// from safe code
    pub unsafe fn from_raw(raw: *mut T::Raw) -> Self {
        Owned {
            object: T::from_raw(raw),
        }
    }

    /// Converts `self` into the underlying harfbuzz object pointer value. The
    /// resulting pointer has to be manually destroyed using `hb_TYPE_destroy`
    /// or be converted back into the wrapper using the `from_raw` function to
    /// avoid leaking memory.
    pub fn into_raw(owned: Owned<T>) -> *mut T::Raw {
        let result = owned.object.as_raw();
        std::mem::forget(owned);
        result
    }

    /// Demotes an `Owned` pointer to a `Shared` pointer.
    ///
    /// Use this method when you don't need exclusive (mutable) access to the
    /// object anymore. For differences between `Owned` and `Shared` pointers
    /// see the documentation on the respective structs.
    ///
    /// Note that `Shared<T>` also implements `From<Owned<T>>` which allows
    /// implicit conversions in many functions.
    #[allow(clippy::wrong_self_convention)] //< backward compatibility is more important than clippy
    pub fn to_shared(self) -> Shared<T> {
        self.into()
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
        &self.object
    }
}

impl<T: HarfbuzzObject> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.object
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::mem;
    use std::rc::Rc;
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

    // this is a mock struct for testing HarfbuzzObject's behaviour.
    #[derive(Debug, Clone)]
    struct ReferenceCounter {
        share_count: Rc<Cell<isize>>,
    }

    unsafe impl HarfbuzzObject for ReferenceCounter {
        type Raw = Cell<isize>;

        unsafe fn from_raw(raw: *const Cell<isize>) -> Self {
            ReferenceCounter {
                share_count: Rc::from_raw(raw as *mut _),
            }
        }

        fn as_raw(&self) -> *mut Cell<isize> {
            Rc::into_raw(self.share_count.clone()) as *mut _
        }

        unsafe fn reference(&self) {
            println!("referencing {:?}", self);
            let rc = self.share_count.get();
            self.share_count.set(rc + 1);
        }

        unsafe fn dereference(&self) {
            println!("dereferencing {:?}", self);
            let rc = self.share_count.get();
            self.share_count.set(rc - 1);
        }
    }

    #[test]
    fn reference_counting_shared() {
        // Mimic a C-API that returns a pointer to a reference counted value.
        let object = ReferenceCounter {
            share_count: Rc::new(Cell::new(1)),
        };

        // this clones the underlying `Rc`
        let raw = object.as_raw();

        // so we expect two shared owners
        assert_eq!(Rc::strong_count(&object.share_count), 2);

        let shared: Shared<ReferenceCounter> = unsafe { Shared::from_raw_owned(raw) };

        assert_eq!(shared.share_count.get(), 1);
        {
            // we create another `Shared` pointer...
            let shared2 = Shared::clone(&shared);
            // which clones
            assert_eq!(shared.share_count.get(), 2);
            mem::drop(shared2);
        }
        assert_eq!(shared.share_count.get(), 1);
        mem::drop(shared);

        assert_eq!(object.share_count.get(), 0);

        // ensure there are no dangling references
        assert_eq!(Rc::strong_count(&object.share_count), 1);
    }
}
