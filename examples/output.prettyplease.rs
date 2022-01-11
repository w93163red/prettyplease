use crate::char;
use crate::fmt::{self, Write};
use crate::mem;
use super::from_utf8_unchecked;
use super::validations::utf8_char_width;
/// Lossy UTF-8 string.
#[unstable(feature = "str_internals", issue = "none")]
pub struct Utf8Lossy {
    bytes: [u8],
}
impl Utf8Lossy {
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> &Utf8Lossy {
        unsafe { mem::transmute(bytes) }
    }
    pub fn chunks(&self) -> Utf8LossyChunksIter<'_> {
        Utf8LossyChunksIter {
            source: &self.bytes,
        }
    }
}
/// Iterator over lossy UTF-8 string
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[unstable(feature = "str_internals", issue = "none")]
#[allow(missing_debug_implementations)]
pub struct Utf8LossyChunksIter<'a> {
    source: &'a [u8],
}
#[unstable(feature = "str_internals", issue = "none")]
#[derive(PartialEq, Eq, Debug)]
pub struct Utf8LossyChunk<'a> {
    /// Sequence of valid chars.
    /// Can be empty between broken UTF-8 chars.
    pub valid: &'a str,
    /// Single broken char, empty if none.
    /// Empty iff iterator item is last.
    pub broken: &'a [u8],
}
impl<'a> Iterator for Utf8LossyChunksIter<'a> {
    type Item = Utf8LossyChunk<'a>;
    fn next(&mut self) -> Option<Utf8LossyChunk<'a>> {
        if self.source.is_empty() {
            return None;
        }
        const TAG_CONT_U8: u8 = 128;
        fn safe_get(xs: &[u8], i: usize) -> u8 {
            *xs.get(i).unwrap_or(&0)
        }
        let mut i = 0;
        let mut valid_up_to = 0;
        while i < self.source.len() {
            let byte = unsafe { *self.source.get_unchecked(i) };
            i += 1;
            if byte < 128 {
            } else {
                let w = utf8_char_width(byte);
                match w {
                    2 => {
                        if safe_get(self.source, i) & 192 != TAG_CONT_U8 {
                            break;
                        }
                        i += 1;
                    }
                    3 => {
                        match (byte, safe_get(self.source, i)) {
                            (0xE0, 0xA0..=0xBF) => {}
                            (0xE1..=0xEC, 0x80..=0xBF) => {}
                            (0xED, 0x80..=0x9F) => {}
                            (0xEE..=0xEF, 0x80..=0xBF) => {}
                            _ => break,
                        }
                        i += 1;
                        if safe_get(self.source, i) & 192 != TAG_CONT_U8 {
                            break;
                        }
                        i += 1;
                    }
                    4 => {
                        match (byte, safe_get(self.source, i)) {
                            (0xF0, 0x90..=0xBF) => {}
                            (0xF1..=0xF3, 0x80..=0xBF) => {}
                            (0xF4, 0x80..=0x8F) => {}
                            _ => break,
                        }
                        i += 1;
                        if safe_get(self.source, i) & 192 != TAG_CONT_U8 {
                            break;
                        }
                        i += 1;
                        if safe_get(self.source, i) & 192 != TAG_CONT_U8 {
                            break;
                        }
                        i += 1;
                    }
                    _ => break,
                }
            }
            valid_up_to = i;
        }
        let (inspected, remaining) = unsafe { self.source.split_at_unchecked(i) };
        self.source = remaining;
        let (valid, broken) = unsafe { inspected.split_at_unchecked(valid_up_to) };
        Some(Utf8LossyChunk {
            valid: unsafe { from_utf8_unchecked(valid) },
            broken,
        })
    }
}
impl fmt::Display for Utf8Lossy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.bytes.is_empty() {
            return "".fmt(f);
        }
        for Utf8LossyChunk { valid, broken } in self.chunks() {
            if valid.len() == self.bytes.len() {
                assert!(broken . is_empty ());
                return valid.fmt(f);
            }
            f.write_str(valid)?;
            if !broken.is_empty() {
                f.write_char(char::REPLACEMENT_CHARACTER)?;
            }
        }
        Ok(())
    }
}
impl fmt::Debug for Utf8Lossy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        for Utf8LossyChunk { valid, broken } in self.chunks() {
            {
                let mut from = 0;
                for (i, c) in valid.char_indices() {
                    let esc = c.escape_debug();
                    if esc.len() != 1 {
                        f.write_str(&valid[from..i])?;
                        for c in esc {
                            f.write_char(c)?;
                        }
                        from = i + c.len_utf8();
                    }
                }
                f.write_str(&valid[from..])?;
            }
            for &b in broken {
                write!(f, "\\x{:02x}", b)?;
            }
        }
        f.write_char('"')
    }
}
