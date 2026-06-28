use std::{collections::BTreeMap, fmt::Display};

/// Possible errors encountered in the becoded input.
#[derive(Debug, thiserror::Error)]
pub enum BenCodeError {
    #[error("invalid string: {0}")]
    InvalidString(String),
    #[error("invalid integer: {0}")]
    InvalidInt(String),
    #[error("invalid list: {0}")]
    InvalidList(String),
    #[error("invalid dictionary: {0}")]
    InvalidDict(String),
}

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
    Int(isize),
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
        let bencode = BenCode::Int(3);
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
}
