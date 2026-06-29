//! Bencode types and parsing.
use std::{collections::BTreeMap, fmt::Display};

use num_bigint::BigInt;

/// Bencoded types.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BenCode {
    /// Strings are length-prefixed base ten followed by a colon and the string. For example 4:spam
    /// corresponds to 'spam'.
    String(Vec<u8>),
    /// Integers are represented by an 'i' followed by the number in base 10 followed by an 'e'. For
    ///  example i3e corresponds to 3 and i-3e corresponds to -3. Integers have no size limitation.
    ///  i-0e is invalid. All encodings with a leading zero, such as i03e, are invalid, other than
    ///  i0e, which of course corresponds to 0.
    Int(BigInt),
    /// Lists are encoded as an 'l' followed by their elements (also bencoded) followed by an 'e'.
    /// For example l4:spam4:eggse corresponds to ['spam', 'eggs'].
    List(Vec<BenCode>),
    /// Dictionaries are encoded as a 'd' followed by a list of alternating keys and their
    /// corresponding values followed by an 'e'. For example, d3:cow3:moo4:spam4:eggse corresponds
    /// to {'cow': 'moo', 'spam': 'eggs'} and d4:spaml1:a1:bee corresponds to {'spam': ['a', 'b']}.
    /// Keys must be strings and appear in sorted order (sorted as raw strings, not alphanumerics).
    Dict(BTreeMap<Vec<u8>, BenCode>),
}

impl Display for BenCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenCode::String(b) => match str::from_utf8(b) {
                Ok(s) => write!(f, "{}:{s}", s.len()),
                Err(_) => Err(std::fmt::Error),
            },
            BenCode::Int(i) => write!(f, "i{i}e"),
            BenCode::List(l) => {
                write!(f, "l")?;
                let _ = l.iter().try_for_each(|el| write!(f, "{el}"));
                write!(f, "e")
            }
            BenCode::Dict(m) => {
                write!(f, "d")?;
                for (k, v) in m.iter() {
                    let _ = match str::from_utf8(k) {
                        Ok(k) => write!(f, "{}:{k}", k.len()),
                        Err(_) => Err(std::fmt::Error),
                    };
                    write!(f, "{v}")?
                }
                write!(f, "e")
            }
        }
    }
}

/// Possible errors encountered in the becoded input.
#[derive(Debug, thiserror::Error)]
pub enum BenCodeError {
    /// Invalid bencode string.
    #[error("invalid string")]
    InvalidString,
    /// Invalid bencode integer.
    #[error("invalid integer")]
    InvalidInt,
    /// Invalid bencode list.
    #[error("invalid list")]
    InvalidList,
    /// Invalid bencode dictionary.
    #[error("invalid dictionary")]
    InvalidDict,
    /// Unexpected encountered byte.
    #[error("unexpected byte: {0}")]
    UnexpectedByte(u8),
    /// Unexpected end of input.
    #[error("input ended unexpectedly")]
    UnexpectedEof,
}

/// Attempt to parse the input bytes as `BenCode`.
///
/// # Errors
///
/// Returns a [`BenCodeError`] in the case of invalid bencoding.
pub fn parse(input: &[u8]) -> Result<BenCode, BenCodeError> {
    BencodeParser::new(input).parse()
}

#[derive(Debug)]
struct BencodeParser<'input> {
    /// Bencoded input.
    input: &'input [u8],
    /// The next byte index to consume.
    idx: usize,
}

impl<'input> BencodeParser<'input> {
    fn new(input: &'input [u8]) -> Self {
        Self { input, idx: 0 }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.input.get(self.idx).copied();
        if byte.is_some() {
            self.idx += 1
        }
        byte
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.idx).copied()
    }

    fn parse(&mut self) -> Result<BenCode, BenCodeError> {
        match self.peek() {
            Some(b'i') => self.parse_int(),
            Some(b'l') => self.parse_list(),
            Some(b'd') => self.parse_dict(),
            Some(b) if b.is_ascii_digit() => self.parse_str(),
            Some(b) => Err(BenCodeError::UnexpectedByte(b)),
            None => Err(BenCodeError::UnexpectedEof),
        }
    }

    fn parse_str(&mut self) -> Result<BenCode, BenCodeError> {
        let len_start = self.idx;
        while let Some(b) = self.peek() {
            if b == b':' {
                break;
            }
            if !b.is_ascii_digit() {
                return Err(BenCodeError::InvalidString);
            }
            self.idx += 1
        }

        if self.peek() != Some(b':') {
            return Err(BenCodeError::InvalidString);
        }

        let len: usize = std::str::from_utf8(&self.input[len_start..self.idx])
            .map_err(|_| BenCodeError::InvalidString)?
            .parse()
            .map_err(|_| BenCodeError::InvalidString)?;

        self.next_byte(); // consume semicolon

        let str_start = self.idx;

        if self.idx + len > self.input.len() {
            return Err(BenCodeError::UnexpectedEof);
        }

        self.idx += len;
        Ok(BenCode::String(self.input[str_start..self.idx].to_vec()))
    }

    fn parse_int(&mut self) -> Result<BenCode, BenCodeError> {
        match self.next_byte() {
            Some(b'i') => {}
            _ => return Err(BenCodeError::InvalidInt),
        }

        let start = self.idx;

        while let Some(b) = self.peek() {
            if b == b'e' {
                break;
            }
            self.idx += 1;
        }

        if self.peek() != Some(b'e') {
            return Err(BenCodeError::InvalidInt);
        }

        let digits = &self.input[start..self.idx];

        self.next_byte(); // consume 'e'

        if digits.is_empty() || digits == b"-0" {
            return Err(BenCodeError::InvalidInt);
        }

        if digits.len() > 1 {
            if digits[0] == b'0' {
                return Err(BenCodeError::InvalidInt);
            }

            if digits[0] == b'-' && digits.get(1) == Some(&b'0') {
                return Err(BenCodeError::InvalidInt);
            }
        }

        let n: BigInt = std::str::from_utf8(digits)
            .map_err(|_| BenCodeError::InvalidInt)?
            .parse()
            .map_err(|_| BenCodeError::InvalidInt)?;

        Ok(BenCode::Int(n))
    }

    fn parse_list(&mut self) -> Result<BenCode, BenCodeError> {
        match self.next_byte() {
            Some(b'l') => {}
            _ => return Err(BenCodeError::InvalidInt),
        }

        let mut items = Vec::new();

        while let Some(b) = self.peek() {
            if b == b'e' {
                break;
            }
            items.push(self.parse()?);
        }

        if self.peek() != Some(b'e') {
            return Err(BenCodeError::InvalidList);
        }

        self.next_byte(); // consume 'e'

        Ok(BenCode::List(items))
    }

    fn parse_dict(&mut self) -> Result<BenCode, BenCodeError> {
        match self.next_byte() {
            Some(b'd') => {}
            _ => return Err(BenCodeError::InvalidDict),
        }

        let mut bt = BTreeMap::new();

        while let Some(b) = self.peek() {
            if b == b'e' {
                break;
            }

            let key = match self.parse()? {
                BenCode::String(s) => s,
                _ => return Err(BenCodeError::InvalidDict),
            };

            let value = self.parse()?;
            if bt.insert(key, value).is_some() {
                return Err(BenCodeError::InvalidDict);
            }
        }

        if self.peek() != Some(b'e') {
            return Err(BenCodeError::InvalidDict);
        }

        self.next_byte(); // consume 'e'

        Ok(BenCode::Dict(bt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_string_bencode_display() {
        let bencode = BenCode::String("spam".into());
        assert_eq!(format!("{bencode}").as_str(), "4:spam");
    }

    #[test]
    fn simple_int_bencode_display() {
        let bencode = BenCode::Int(3.into());
        assert_eq!(format!("{bencode}").as_str(), "i3e");
    }

    #[test]
    fn simple_list_bencode_display() {
        let bencode = BenCode::List(vec![
            BenCode::String("spam".into()),
            BenCode::String("eggs".into()),
        ]);
        assert_eq!(format!("{bencode}").as_str(), "l4:spam4:eggse");
    }

    #[test]
    fn simple_dict_bencode_display() {
        let mut bt = BTreeMap::new();
        bt.insert(b"cow".to_vec(), BenCode::String("moo".into()));
        bt.insert(b"spam".to_vec(), BenCode::String("eggs".into()));
        let bencode = BenCode::Dict(bt);
        assert_eq!(format!("{bencode}").as_str(), "d3:cow3:moo4:spam4:eggse");
    }

    #[test]
    fn simple_dict_bencode_with_list_val_display() {
        let mut bt = BTreeMap::new();
        bt.insert(
            b"spam".to_vec(),
            BenCode::List(vec![
                BenCode::String("a".into()),
                BenCode::String("b".into()),
            ]),
        );
        let bencode = BenCode::Dict(bt);
        assert_eq!(format!("{bencode}").as_str(), "d4:spaml1:a1:bee");
    }

    #[test]
    fn parse_valid_simple_dict() {
        let input = b"d4:spaml1:a1:bee";
        let mut bt = BTreeMap::new();
        bt.insert(
            b"spam".to_vec(),
            BenCode::List(vec![
                BenCode::String("a".into()),
                BenCode::String("b".into()),
            ]),
        );
        let expected = BenCode::Dict(bt);
        let parsed = BencodeParser::new(input).parse().expect("valid bencode");
        assert_eq!(expected, parsed)
    }

    #[test]
    fn parse_bencode_str() {
        let input = b"4:spam";
        let parsed = BencodeParser::new(input).parse().expect("valid bencode");
        assert_eq!(parsed, BenCode::String(b"spam".to_vec()))
    }

    #[test]
    fn invalid_bencode_int() {
        let input = b"i-0e";
        assert!(BencodeParser::new(input).parse().is_err());
    }

    #[test]
    fn parse_valid_bencode_int() {
        let input = b"i-3e";
        let parsed = BencodeParser::new(input).parse().expect("valid bencode");
        assert_eq!(parsed, BenCode::Int((-3).into()))
    }
}
