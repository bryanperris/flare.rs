use super::hog::HogEntry;


pub trait GameFile {
    fn get_data(&self) -> &[u8];
}

pub trait GameFilesystem {
    fn find_file(&self, name: &str) -> Option<&dyn GameFile>;
}

pub trait GameFilesystemWithHogs {
    fn find_file_in_hog<'hog>(&self, name: &str) -> Option<HogGameFile>;
}

pub struct HogGameFile<'hog> {
    associated_hog_entry: &'hog HogEntry
}

impl<'hog> GameFile for HogGameFile<'hog> {
    fn get_data(&self) -> &[u8] {
        &self.associated_hog_entry.data
    }
}

// impl GameFilesystem {
//     fn get_gamefile_from_hog(&self, name: &str) -> Option<&[u8]> {
//         for lib in &self.libraries {
//             if lib.borrow_entries().contains_key(name) {
//                 return Some(&lib.borrow_entries()[name].data)
//             }
//         }

//         None
//     }
// }