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

#[cfg(test)]
mod tests {
    use super::*;

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
