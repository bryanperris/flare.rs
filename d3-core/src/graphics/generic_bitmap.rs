use crate::string::D3String;

use super::bitmap::{Bitmap16, BitmapFlags};

#[derive(Debug, Clone)]
pub struct GenericBitmap16 {
    data: Vec<u16>,
    width: usize,
    height: usize,
    name: D3String,
}

impl GenericBitmap16 {
    pub fn new(data: Vec<u16>, width: usize, height: usize) -> Self {
        Self {
            data: data,
            width: width,
            height: height,
            name: D3String::new()
        }
    }
}

impl Bitmap16 for GenericBitmap16 {
    fn data(&self) -> &[u16] {
        &self.data
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn mip_levels(&self) -> usize {
        0
    }

    fn flags(&self) -> &super::bitmap::BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &crate::string::D3String {
        &self.name
    }

    fn format(&self) -> super::bitmap::BitmapFormat {
        super::bitmap::BitmapFormat::Fmt4444
    }

    fn make_funny(&mut self) {
        todo!()
    }
}