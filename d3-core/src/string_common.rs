use anyhow::anyhow;
use anyhow::Result;

pub fn parse_raw_string(bytes: &[u8]) -> Option<&str> {
    // Find the position of the null byte
    if let Some(pos) = bytes.iter().position(|&x| x == 0) {
        // Create a slice up to the null byte
        let trimmed_bytes = &bytes[..pos];

        // Convert the slice to a &str and return it
        std::str::from_utf8(trimmed_bytes).ok()
    } else {
        // If no null byte is found, return the whole string if it's valid UTF-8
        std::str::from_utf8(bytes).ok()
    }
}

pub fn to_uppercase_ascii(value: i32) -> i32 {
    // Convert the i32 to a char
    if let Some(c) = char::from_u32(value as u32) {
        // Convert the char to uppercase
        let upper_c = c.to_ascii_uppercase();
        // Convert the uppercase char back to i32
        upper_c as i32
    } else {
        // Handle the case where the value is not a valid ASCII character
        value
    }
}

pub fn convert_to_ascii_slice(string: &str) -> Result<Box<[u8]>> {
    let r: Result<Vec<u8>, ()> = string.chars().map(|c| {
        if c.is_ascii() {
            Ok(c as u8)
        } else {
            Err(())
        }
    }).collect();

    match r {
        Ok(ascii) => {
            return Ok(ascii.into_boxed_slice());
        },
        Err(_) => {
            return Err(anyhow!("Found non-ascii values in string!"))
        }
    }
}