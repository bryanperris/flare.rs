use std::fmt;
use std::ops::{Index, IndexMut, Range, RangeFrom};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3String {
    data: Vec<u8>,
    size_constraint: Option<usize>,
}

pub static EMPTY: D3String = D3String {
    data: Vec::new(),
    size_constraint: None,
};

impl Default for D3String {
    fn default() -> Self {
        Self { 
            data: vec![0],
            size_constraint: None
        }
    }
}

impl D3String {

    // Create a new, empty D3String with no size constraint
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            size_constraint: None,
        }
    }

    // Create a new, empty D3String with a size constraint
    pub fn with_fixed_sized(size_constraint: usize) -> Self {
        Self {
            data: vec![0u8; size_constraint],
            size_constraint: Some(size_constraint),
        }
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            data: slice.to_vec(),
            size_constraint: Some(slice.len())
        }
    }

    // Create a D3String from a &str until a terminating byte is found
    pub fn from_str_until(s: &str, terminator: u8, size_constraint: Option<usize>) -> Self {
        let mut data = Vec::new();
        for &byte in s.as_bytes() {
            if byte == terminator || (size_constraint.is_some() && (data.len() + 1) >= size_constraint.unwrap()) {
                data.push(b'\0');
                break;
            }
            data.push(byte);
        }
        D3String { data, size_constraint }
    }

    pub fn to_string(&self) -> Result<String, std::string::FromUtf8Error> {
        if let Some(pos) = self.data.iter().position(|&x| x == b'\0') {
            String::from_utf8(self.data[..pos].to_vec())
        } else {
            String::from_utf8(self.data.clone())
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        if self.size_constraint.is_some() {
            self.data[0] == b'\0'
        }
        else {
            self.data.is_empty()
        }
    }

    // Append a &str to the D3String, respecting the size constraint
    pub fn push_str(&mut self, s: &str) {
        let available_space = self.size_constraint.map_or(usize::MAX, |max| max.saturating_sub(self.data.len()));
        let bytes_to_add = s.as_bytes().iter().take(available_space).collect::<Vec<_>>();
        self.data.extend(bytes_to_add.into_iter().cloned());
    }

    // Clear the D3String
    pub fn clear(&mut self) {
        self.data.clear();

        if self.size_constraint.is_some() {
            self.data = vec![0u8; self.size_constraint.unwrap()];
        }
    }

    pub fn iter(&self) -> D3StringIterator {
        D3StringIterator {
            string: self,
            index: 0
        }
    }

    pub fn to_owned(&self) -> Self {
        Self {
            data: self.data.to_owned(),
            size_constraint: self.size_constraint
        }
    }

    pub fn char_at(&self, index: usize) -> char {
        self.data[index] as char
    }

    pub fn byte_at(&self, index: usize) -> u8 {
        self.data[index]
    }
}

// impl From<&str> for D3String {
//     fn from(s: &str) -> Self {
//         D3String::from_str_until(s, 0, None) // Use 0 as the default terminator and no size constraint
//     }
// }

impl From<String> for D3String {
    fn from(s: String) -> Self {
        D3String::from_str_until(&s, 0, None) // Use 0 as the default terminator and no size constraint
    }
}

impl From<D3String> for String {
    fn from(d3s: D3String) -> Self {
        d3s.to_string().unwrap()
    }
}

impl From<&D3String> for String {
    fn from(d3s: &D3String) -> Self {
        d3s.to_string().unwrap()
    }
}

impl From<&'static str> for D3String {
    fn from(s: &'static str) -> Self {
        D3String::from(s.to_string())
    }
}

impl fmt::Display for D3String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_string() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "<invalid UTF-8>"),
        }
    }
}

impl Index<usize> for D3String {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for D3String {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

pub struct D3StringIterator<'a> {
    string: &'a D3String,
    index: usize
}

impl<'a> Iterator for D3StringIterator<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.string.data.len() {
            let item = &self.string.data[self.index];
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl Index<Range<usize>> for D3String {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.data[index]
    }
}

impl Index<RangeFrom<usize>> for D3String {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<Range<usize>> for D3String {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl Hash for D3String {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

#[cfg(test)]
pub mod tests {
    use std::{env, fs::{File}, path::{Path, PathBuf}};
    use function_name::named;

    use super::*;

    #[test]
    fn d3string_cmp() {
        crate::test_common::setup();

        let a: D3String = "Hello!".into();
        let b: D3String = "Hello!".into();
        let c: D3String = "askldjalkdsjasldk".into();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}