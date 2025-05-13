
use core::borrow;
use std::{collections::HashMap, io::{BufReader, Read, Seek}};

use anyhow::Result;

use crate::string::D3String;

mod internal {
    /* Internals used for reading/writing the hog raw data */

    /*	HOG FILE FORMAT v2.0

                HOG_TAG_STR			[strlen()]
                NFILES				[int32]
                HDRINFO				[HOG_HDR_SIZE - 4]
                FILE_TABLE			[sizeof(FILE_ENTRY) * NFILES]
                FILE 0				[filelen(FILE 0)]
                FILE 1				[filelen(FILE 1)]
                .
                .
                .
                FILE NFILES-1		[filelen(NFILES -1)]
    */

    use core::{num, ptr::read};
    use std::io::{BufReader, Read, Seek};
    use anyhow::Result;
    use byteorder::{LittleEndian, ReadBytesExt, BigEndian};
    use anyhow::Context;

    use crate::{filesystem::hog::HogEntry, string::D3String};

    use super::Hog;

    const HEADER_SIZE: usize = 64;
    const MAGIC: &str = "HOG2";
    const HOG_FILENAME_SIZE: usize = 36;

    struct HogFileEntry {
        name: D3String,
        flags: u32,
        size: usize,
        timestamp: u32,
    }
    
    #[derive(Debug)]
    enum HogError {
        IncorrectFileCount,
        NoMemory,
    }

   pub(crate) fn new<R: Read + Seek>(name: String, reader: &mut BufReader<R>) -> Result<Hog> {
        let mut magic = [0u8; MAGIC.len()];
        reader.read_exact(&mut magic).context("Failed to read magic")?;
        let magic_str = std::str::from_utf8(&magic).unwrap();

        trace!("Hog magic: {}", magic_str);

        let mut hog = Hog::default();
        hog.name = name;

        let num_entries = reader.read_u32::<LittleEndian>().unwrap();
        let mut header_info = [0u8; HEADER_SIZE - 4]; // NFILES is part of the header
        reader.read_exact(&mut header_info).context("Failed to read header info")?;

        // Read the table
        let mut table: Vec<HogFileEntry> = Vec::default();
        for _ in 0..num_entries {
            let mut entry_name = [0u8; HOG_FILENAME_SIZE];
            reader.read_exact(&mut entry_name).context("Failed to read entry name")?;

            let entry_header = HogFileEntry {
                name: D3String::from_slice(&entry_name),
                flags: reader.read_u32::<LittleEndian>().unwrap(),
                size: reader.read_u32::<LittleEndian>().unwrap() as usize,
                timestamp: reader.read_u32::<LittleEndian>().unwrap()
            };

            trace!("entry name: {}", entry_header.name);

            table.push(entry_header);
        }

        for entry in table.iter() {
            let mut entry_data = vec![0u8; entry.size];
            reader.read_exact(&mut entry_data).context("Failed to read entry data")?;

            /* Add the entry to the hog */
            hog.entries.insert(entry.name.to_string().unwrap(), HogEntry {
                flags: entry.flags,
                data: entry_data.as_slice().into()
            });
        }

        Ok(hog)

   }

   // TODO: Hog file writer
}


pub struct HogEntry {
    pub flags: u32,
    pub data: Box<[u8]>
}

/// Implements Descent 3 hog, spec 2.0
pub struct Hog {
    name: String,
    entries: HashMap<String, HogEntry>,
}

impl std::fmt::Display for Hog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Hog {{ name: {} }}", self.name)?;
        
        for (k, v) in &self.entries {
            writeln!(f, "Entry: {}", k)?;
            writeln!(f, " size: {}", v.data.len())?;
        }

        Ok(())
    }
}

impl Default for HogEntry {
    fn default() -> Self {
        Self { 
            flags: Default::default(), 
            data: Default::default() 
        }
    }
}

impl Default for Hog {
    fn default() -> Self {
        Self { 
            name: Default::default(), 
            entries: Default::default() 
        }
    }
}

impl Hog {
    pub fn new(name: String) -> Self {
        let mut hog = Hog::default();
        hog.name = name;
        hog
    }

    pub fn new_from_stream<R: Read + Seek>(reader: &mut BufReader<R>, name: String) -> Result<Self> {
        internal::new(name, reader)
    }

    pub fn borrow_entries(&self) -> &HashMap<String, HogEntry> {
        &self.entries
    }

    pub fn borrow_entries_mut(&mut self) -> &mut HashMap<String, HogEntry> {
        &mut self.entries
    }
}

#[cfg(test)]
pub mod tests {
    use std::{env, fs::{File}, path::{Path, PathBuf}};
    use function_name::named;

    use crate::{assert_md5, display_1555, display_4444, testdata};

    use super::*;

    #[test]
    #[named]
    fn hog_read_test() {
        crate::test_common::setup();

        let name = "test.hog";
        let testhog_file = File::open(testdata!(name)).unwrap();
        let mut reader = BufReader::new(testhog_file);
        let testhog = Hog::new_from_stream(&mut reader, name.to_string()).unwrap();

        println!("{}", testhog);
        
        assert_md5!( 
            testhog.borrow_entries()["badapple_1555_1mm.ogf"].data.to_vec(),
            "9c322cadc8f0472fe40beeff8ad65b02"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple_1555_5mm.ogf"].data.to_vec(),
            "43523a8c916fc97df098ddcd4f3b85d3"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple-219frames.iff"].data.to_vec(),
            "2da28eaa2bee1e0edee5a217684f0dbb"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple_4444_1mm.ogf"].data.to_vec(),
            "29a4a6e66b2c0721242b96313358edfd"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple_4444_5mm.ogf"].data.to_vec(),
            "879aa76daafb7622470f00df69e45dec"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple.pcx"].data.to_vec(),
            "38a94bb148e3953b8649e6b56aec0e9b"
        );

        assert_md5!( 
            testhog.borrow_entries()["badapple.tga"].data.to_vec(),
            "9b7b1cbc52635c8735318da3e4383ce0"
        );

        assert_md5!( 
            testhog.borrow_entries()["fake_ani.oaf"].data.to_vec(),
            "ea2b83b87d85852e45d9247b68526372"
        );

        assert_md5!( 
            testhog.borrow_entries()["fake_dll.dll"].data.to_vec(),
            "32b3ca016325e6e727285f0ac7a4bd70"
        );

        assert_md5!( 
            testhog.borrow_entries()["fake_gam.gam"].data.to_vec(),
            "458c8f1506a91596fd01004ea62ef654"
        );
    }
}