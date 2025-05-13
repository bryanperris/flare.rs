pub mod image_format_iff;
pub mod image_format_ogf;
pub mod image_format_pcx;
pub mod videoclip;


use std::io::BufReader;

use anyhow::Result;

use bitflags::bitflags;

use crate::{create_rng, graphics::bitmap, string::D3String};

// TODO: Some of these bitmap system flags need to be seperate from the bitmap resources

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BitmapFlags: u8 {
        const None  =        0b00000000;
        const Transparent =  0b00000001;
        /// this bitmap has changed since last frame (useful for hardware cacheing)
        const Changed =      0b00000010;
        /// This bitmap has mip levels
        const MipMapped =    0b00000100;
        /// This bitmap is not paged in
        const NonResident =  0b00001000;
        /// Calculate mip levels when this bitmap is paged in
        const WantsMip  =    0b00010000;
        /// Read data as 4444 when this bitmap is paged in
        const Wants4444  =   0b00100000;
        /// This bitmap was just allocated and hasn't been to the video card
        const BrandNew  =    0b01000000;
        /// This bitmap is compressable for 3dhardware that supports it
        const Compressable = 0b10000000;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BitmapFormat {
    Fmt1555,
    Fmt4444
}

pub trait Bitmap16: std::fmt::Debug {
    fn data(&self) -> &[u16];
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn mip_levels(&self) -> usize;
    fn flags(&self) -> &BitmapFlags;
    fn name(&self) -> &D3String;
    fn format(&self) -> BitmapFormat; // Should be just something for TGA only
    fn make_funny(&mut self);
}

pub(crate) trait ScaleableBitmap16 {
    fn new_scaled_data(&mut self, data: Box<[u16]>, w: usize, h: usize); // This should set changed
}

/* TODO: Rather use lifetime managed references to the original bitmap... */

// pub struct BitmapChunk16<'bitmap> {
//     square_size: usize,
//     bitmap_ref: &'bitmap dyn Bitmap16
// }

#[derive(Debug, Clone)]
pub struct BitmapChunk16 {
    square_size: usize,
    data: Vec<u16>,
    format: BitmapFormat,
    name: D3String,
}

impl Bitmap16 for BitmapChunk16 {
    fn data(&self) -> &[u16] {
        &self.data
    }

    fn width(&self) -> usize {
        self.square_size
    }

    fn height(&self) -> usize {
        self.square_size
    }

    fn mip_levels(&self) -> usize {
        0
    }

    fn flags(&self) -> &BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &D3String {
        &self.name
    }

    fn format(&self) -> BitmapFormat {
        self.format
    }

    fn make_funny(&mut self) {
        match self.format() {
            BitmapFormat::Fmt4444 => {
                for i in 0..self.square_size * 2 {
                    self.data[i] = generate_random_color_4444();
                }
            },
            BitmapFormat::Fmt1555 => {
                for i in 0..self.square_size * 2 {
                    self.data[i] = generate_random_color_1555();
                }
            },
            _ => {}
        }
    }
}

impl BitmapChunk16 {
    fn new(size: usize, format: BitmapFormat, name: D3String) -> Self {
        BitmapChunk16 {
            square_size: size,
            format: format,
            name: name,
            data: vec![0u16; size * 2]
        }
    }
}


/* TODO: register these kind of bitmaps into the bitmap cache */
pub struct ChunkedBitmap16 {
    pixel_width: usize,
    pixel_height: usize,
    width: usize, // in square bitmaps
    height: usize, // in square bitmaps
    bitmaps: Vec<BitmapChunk16>
}

impl dyn Bitmap16 {
    pub fn into_chunked(&self) -> Result<ChunkedBitmap16> {

        /* Find the smallest dimension and base it off that */
        let smallest = std::cmp::min(self.width(), self.height());

        let dim = match smallest {
            v if v <= 32 => 32.0,
            v if v <= 64 => 64.0,
            _v => 128.0f32
        };

        let divided_width = self.width() as f32 / dim;
        let divided_height = self.height() as f32 / dim;

        // Get how many pieces we need across and down
        let count_across = match divided_width - divided_width.trunc() {
            v if v > 0.0 => divided_width.trunc() + 1.0,
            _v => divided_width.trunc()
        } as usize;

        let count_down = match divided_height - divided_height.trunc() {
            v if v > 0.0 => divided_width.trunc() + 1.0,
            _v => divided_width.trunc()
        } as usize;

        assert!(count_across > 0);
        assert!(count_down > 0);

        let mut bitmaps:Vec<BitmapChunk16> = Vec::default();
        let size = dim.trunc() as usize;

        for i in 0..(count_across * count_down) {
            let name = D3String::from(format!("{}-chunk-{}", self.name(), i));

            bitmaps.push(
                BitmapChunk16::new(size, self.format(), name)
            );
        }

        let shift = match size {
            32 => 5,
            64 => 6,
            128 => 7,
            _ => return Err(anyhow!("get jeff!!"))
        };

        for h_index in 0..count_down {
            for w_index in 0..count_across {
                let x_start = w_index << shift;
                let y_start = h_index << shift;

                /* loop through the chunks, look for end x,y */
                let x_max = if w_index < count_across - 1 { size } else { self.width() - x_start};
                let y_max = if h_index < count_down - 1 { size } else { self.height() - y_start};

                let src_offset = y_start * self.width() + x_start;
                let dst_offset = h_index * count_down + w_index;
                let src = &self.data()[src_offset .. self.data().len() - y_start];
                let dst = bitmaps[dst_offset].data.as_mut_slice();

                let mut s = 0;
                let mut d = 0;

                for _ in 0..y_max {
                    for x in 0..x_max {
                        dst[d + x] = src[s + x];
                    }

                    s += self.width() as usize;
                    d += size;
                }
            }
        }

        Ok(ChunkedBitmap16 {
            pixel_width: self.width(),
            pixel_height: self.height(),
            height: size,
            width: size,
            bitmaps: bitmaps
        })
    }
}

pub fn generate_random_color_4444() -> u16 {
    extern crate tinyrand;

    use tinyrand::{Rand, StdRand};

    let mut rand = create_rng();

    let alpha = rand.next_u32() % 16;  // 4 bits
    let red = rand.next_u32() % 16;    // 4 bits
    let green = rand.next_u32() % 16;  // 4 bits
    let blue = rand.next_u32() % 16;   // 4 bits
    
    ((alpha << 12) | (red << 8) | (green << 4) | blue) as u16
}

pub fn generate_random_color_1555() -> u16 {
    extern crate tinyrand;

    use tinyrand::{Rand, StdRand};

    let mut rand = create_rng();
    let alpha = 1;   // 1 bit
    let red = rand.next_u32() % 32;    // 5 bits
    let green = rand.next_u32() % 32;  // 5 bits
    let blue = rand.next_u32() % 32;   // 5 bits
    
    ((alpha << 15) | (red << 10) | (green << 5) | blue) as u16
}

pub fn scale_bitmap_16<B: Bitmap16 + Clone + ScaleableBitmap16>(bitmap: &B, mipped: bool, new_w: usize, new_h: usize, additonal_mem: usize) -> Result<B> {
    let original_data = bitmap.data();
    let source_mipped = bitmap.mip_levels() > 0;
    let mut limit = 0;
    let mut new_bitmap = bitmap.clone();
    let mut new_buffer = vec![0u16; (new_w * new_h) + additonal_mem];
    
    if source_mipped && !mipped {
        return Err(anyhow!("Destination bitmap must be mipped"));
    }

    if bitmap.width() == new_w && bitmap.height() == new_h {
        return Ok(new_bitmap);
    }

    for m in 0..bitmap.mip_levels() {
        let src = original_data;
        let dst = new_buffer.as_mut_slice();

        // These are our interpolant variables
        let x_step = bitmap.width() as f32 / new_w as f32;
        let y_step = bitmap.height() as f32 / new_h as f32;
        let mut x_off = 0.0f32;
        let mut y_off = 0.0f32;

        for i in 0..new_h {
            x_off = 0.0;
            for t in 0..new_w {
                dst[i * new_w + t] = src[y_off.trunc() as usize * bitmap.width() + x_off.trunc() as usize];
                x_off += x_step;
            }
            y_off += y_step;
        }
    }

    new_bitmap.new_scaled_data(new_buffer.into_boxed_slice(), new_w, new_h);

    Ok(new_bitmap)
}

#[derive(Debug, Clone)]
pub struct MemBitmap16 {
    data: Vec<u16>,
    width: usize,
    height: usize,
    name: D3String,
}

impl MemBitmap16 {
    pub fn new(w: usize, h: usize) -> Self {
        MemBitmap16 {
            data: Vec::with_capacity(w * h),
            width: w,
            height: h,
            name: "".into()
        }
    }
}

impl Bitmap16 for MemBitmap16 {
    fn data(&self) -> &[u16] {
        &self.data.as_slice()
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

    fn flags(&self) -> &BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &D3String {
        &self.name
    }

    fn format(&self) -> BitmapFormat {
        BitmapFormat::Fmt4444
    }

    fn make_funny(&mut self) {
        todo!()
    }
}

// These functions seem to be related to the editor
// TODO: bm_SaveBitmapTGA
// TODO: bm_CreateChunkedBitmap
// TODO: bm_ChangeSize
// TODO: bm_pixel_transparent
// TODO: bm_rowsize
// TODO: bm_GenerateMipMaps
// TOOO: clear bitmap
// TODO: bm_SetBitmapIfTransparent