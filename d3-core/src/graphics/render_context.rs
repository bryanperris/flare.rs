// TODO:

// TODO: Hashmap of loaded Bitmap16 bitmaps (key is bitmap name)

/*
TOOD: 
initialize lightmaps
Initialize bumpmaps
 */

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::bitmap::{self, Bitmap16};
use super::bumpmap::BumpMap16;
use super::lightmap::LightMap16;
use anyhow::Result;

// pub trait CachedBitmap<T> {
// }

type BitmapEntry= dyn Bitmap16;

// TODO: We want traits for generic bumpmaps and lightmaps
pub struct RenderContext {
    bitmap_cache: HashMap<String, Box<BitmapEntry>>,
    bumpmap_cache: Vec<BumpMap16>,
    lightmap_cache: Vec<LightMap16>
}

// TODO: We to be able to access the bitmap cache based on name or slot number
// We will know better on what to do when we futher in development

impl RenderContext {
    pub fn insert_bitmap(&mut self, id: String, bitmap: Box<BitmapEntry>) {
        self.bitmap_cache.insert(id, bitmap);
    }

    pub fn find_bitmap(&self, id: String) -> Option<&BitmapEntry> {
        let result = self.bitmap_cache.get(&id);

        match result {
            Some(b) => {
               Some(b.as_ref())
            },
            None => None
        }
    }

    pub fn find_bitmap_mut(&mut self, id: String) -> Option<&mut BitmapEntry> {
        let result = self.bitmap_cache.get_mut(&id);

        match result {
            Some(b) => {
               Some(b.as_mut())
            },
            None => None
        }
    }

    pub fn bitmap_exists(&self, id: String) -> bool {
        match self.find_bitmap(id) {
            Some(_) => true,
            _ => false
        }
    }

    pub fn change_end_name(id: &mut String, new_name: String) {
        todo!("Won't support this method!");

        // We shouldn't allow extensions names into the bitmap
        // Nor allow changing anything of the bitmap allocated in memory
    }

    pub fn make_funny_bitmap(&mut self, id: String) {
        let mut bitmap = self.find_bitmap_mut(id);

        match bitmap {
            Some(b) => {
                b.make_funny();

                // TODO: need to set the update bit
            },
            _ => {}
        }
    }

    // XXX: bm_AllocLoadFileBitmap
    // This function feels like something used by the editor
    // Force the client side to deal with opening a data stream
}