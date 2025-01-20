//! Multilocalized unicode objects.

use std::collections::HashMap;
use std::fmt;

fn str_to_16(s: &str) -> u16 {
    let mut bytes = s.bytes();
    ((bytes.next().unwrap() as u16) << 8) | bytes.next().unwrap() as u16
}

fn str_from_16(n: u16) -> String {
    String::from_utf8_lossy(&[(n >> 8) as u8, n as u8]).into()
}

type Lang = u16;
type Country = u16;

/// A multilocalized unicode object.
#[derive(Clone, PartialEq, Eq)]
pub struct Mlu {
    pub(crate) entries: HashMap<(Lang, Country), String>,
}

impl Mlu {
    /// Creates a new MLU.
    pub fn new() -> Mlu {
        Mlu {
            entries: HashMap::new(),
        }
    }

    /// Inserts an entry.
    pub fn insert(&mut self, lang: &str, country: &str, string: String) {
        self.insert_raw(str_to_16(lang), str_to_16(country), string);
    }

    /// Inserts a raw entry.
    pub fn insert_raw(&mut self, lang: u16, country: u16, string: String) {
        self.entries.insert((lang, country), string);
    }

    /// Returns an entry.
    pub fn get(&self, lang: &str, country: &str) -> Option<&str> {
        self.get_raw(str_to_16(lang), str_to_16(country))
    }

    /// Returns a raw entry.
    pub fn get_raw(&self, lang: u16, country: u16) -> Option<&str> {
        self.entries.get(&(lang, country)).map(|x| &**x)
    }

    /// Removes an entry.
    pub fn remove(&mut self, lang: &str, country: &str) -> Option<String> {
        self.remove_raw(str_to_16(lang), str_to_16(country))
    }

    /// Removes a raw entry.
    pub fn remove_raw(&mut self, lang: u16, country: u16) -> Option<String> {
        self.entries.remove(&(lang, country))
    }
}

impl fmt::Debug for Mlu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mlu {{")?;
        for (k, v) in &self.entries {
            write!(f, "({}-{}): {:?}", str_from_16(k.0), str_from_16(k.1), v)?;
        }
        write!(f, "}}")
    }
}
