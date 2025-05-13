use std::{fs::read, io::{BufReader, Read, Seek, SeekFrom}, ops::Deref, ptr};
use crate::{gr_rgb16, graphics::{NEW_TRANSPARENT_COLOR, OPAQUE_FLAG}, string::D3String};
use super::{generate_random_color_1555, generate_random_color_4444, Bitmap16, BitmapFlags, BitmapFormat};
use anyhow::{Context, Error};
use byteorder::{LittleEndian, ReadBytesExt, BigEndian};
use anyhow::Result;

// TODO: bm_page_in_file won't be done here
// We want the modern bitmap system to handle the page, but use 


#[derive(Debug, Clone, Copy)]
pub enum OutrageGraphicsFormat {
    Unknown,
    Compressed,
    Compressed4444Mipped,
    Compressed1555Mipped,
    CompressedNewMipped,
    CompressedMipped ,
    Compressed8bit,
    OutrageTga,
    CompressedTga,
    UncompressedTga
}

impl OutrageGraphicsFormat {
    pub fn new(value: u8) -> Self {
        match value {
            121 => OutrageGraphicsFormat::Compressed4444Mipped,
            122 => OutrageGraphicsFormat::Compressed1555Mipped,
            123 => OutrageGraphicsFormat::CompressedNewMipped,
            124 => OutrageGraphicsFormat::CompressedMipped,
            125 => OutrageGraphicsFormat::Compressed8bit,
            126 => OutrageGraphicsFormat::OutrageTga,
            127 => OutrageGraphicsFormat::Compressed,
            10 => OutrageGraphicsFormat::CompressedTga,
            2 => OutrageGraphicsFormat::UncompressedTga,
            _ => OutrageGraphicsFormat::Unknown
        }
    }

    pub fn is_outrage_type(image_type: u8) -> bool {
        match OutrageGraphicsFormat::new(image_type) {
            OutrageGraphicsFormat::Compressed4444Mipped => return true,
            OutrageGraphicsFormat::Compressed1555Mipped => return true,
            OutrageGraphicsFormat::CompressedNewMipped => return true,
            OutrageGraphicsFormat::CompressedMipped => return true,
            OutrageGraphicsFormat::Compressed8bit => return true,
            OutrageGraphicsFormat::Compressed => return true,
            _ => return false
        }
    }
}

#[derive(Debug, Clone)]
pub struct OgfBitmap {
    width: usize,
    height: usize,
    format: BitmapFormat,
    name: D3String,
    flags: BitmapFlags,
    data: Vec<u16>,
    mipmap_count: usize,
}

impl OgfBitmap {
    fn get_mipmap_width(&self, level: usize) -> usize {
        if self.flags & BitmapFlags::MipMapped != BitmapFlags::MipMapped {
            return self.width;
        }
        else {
            return self.width >> level;
        }
    }

    fn get_mipmap_height(&self, level: usize) -> usize {
        if self.flags & BitmapFlags::MipMapped != BitmapFlags::MipMapped {
            return self.height;
        }
        else {
            return self.height >> level;
        }
    }

    fn get_mipmap_size(&self, level: usize) -> usize {
        self.get_mipmap_width(level) * self.get_mipmap_height(level)
    }

    fn is_mipped(&self) -> bool {
        self.mipmap_count > 1 && self.flags & BitmapFlags::MipMapped == BitmapFlags::MipMapped
    }

    fn compute_mipmap_count(&self) -> usize {
        if self.mipmap_count != 0 {
            return self.mipmap_count;
        }

        if self.flags & BitmapFlags::MipMapped != BitmapFlags::MipMapped {
            return 0;
        }

        let mut w = self.width;
        let mut levels = 0;

        while w > 0 {
            levels += 1;
            w >>= 1;
        }

        return levels;
    }

    fn get_mipped_data_slice_mut(&mut self, level: usize) -> &mut [u16] {
        let mut data_offset = 0;
        let mut size = self.get_mipmap_size(0);

        if level >= 1 && !self.is_mipped() {
            panic!("Cannot access mipmap data of a non-mipped bitmap!");
        }

        for i in 1..(level + 1) {
            data_offset += size;
            size = self.get_mipmap_size(i);
        }

        &mut self.data[data_offset .. (data_offset + size)]
    }

    fn get_mipped_data_slice(&self, level: usize) -> &[u16] {
        let mut data_offset = 0;
        let mut size = self.get_mipmap_size(0);

        if level >= 1 && !self.is_mipped() {
            panic!("Cannot access mipmap data of a non-mipped bitmap!");
        }

        for i in 1..(level + 1) {
            data_offset += size;
            size = self.get_mipmap_size(i);
        }

        &self.data[data_offset .. (data_offset + size)]
    }
}

impl Bitmap16 for OgfBitmap {
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
        self.mipmap_count
    }

    fn flags(&self) -> &BitmapFlags {
        &self.flags
    }

    fn name(&self) -> &D3String {
        &self.name
    }
    
    fn format(&self) -> BitmapFormat {
        self.format
    }
    
    fn make_funny(&mut self) {
        let image_type = self.format;

        let gen_random_color = match image_type {
            BitmapFormat::Fmt1555 => {
                generate_random_color_1555
            },
            _ => generate_random_color_4444
        };

        for m in 0..self.mip_levels() {
            let mut data = self.get_mipped_data_slice_mut(m);
                
            for i in 0..data.len() {
                data[i] = gen_random_color();
            }
        }
    }
}

impl OgfBitmap {
    // Loads a TGA or OFG into memory
    pub fn new<R: Read + Seek>(reader: &mut BufReader<R>, requested_format: BitmapFormat) -> Result<Self> {
        let read_image_id_length = reader.read_u8().unwrap();
        let read_color_map_type = reader.read_u8().unwrap();
        let read_image_type = reader.read_u8().unwrap();

        // trace!("color map type: {}", read_color_map_type);

        if read_color_map_type != 0 || !OutrageGraphicsFormat::is_outrage_type(read_image_type) {
            return Err(anyhow!("Can't read this type of TGA"));
        }

        let outrage_image_type = OutrageGraphicsFormat::new(read_image_type);

        match outrage_image_type {
            OutrageGraphicsFormat::Compressed8bit =>
                return Err(anyhow!("compressed 8-bit is not supported!")),
            _ => {}
        };

        trace!("outrage type: {:?}", outrage_image_type);

        let mut read_name = [0u8; 35];
        let _ = reader.read(read_name.as_mut_slice());
        let mut name = "".to_string();

        // TODO: We really should ignore this in the bitmap system, go by actual filenames

        if let Some(pos) = read_name.iter().position(|&c| c == 0) {
            let valid_data = &read_name[..pos];
            name = std::str::from_utf8(&valid_data).context("Failed to parse ut8 for name").unwrap().to_owned();

            match outrage_image_type {
                OutrageGraphicsFormat::Compressed4444Mipped |
                OutrageGraphicsFormat::Compressed1555Mipped |
                OutrageGraphicsFormat::CompressedNewMipped => {
                    // For these formats pull back the stream position to byte after the terminating 0
                    let _ = reader.seek(SeekFrom::Current(-((read_name.len() - pos - 1) as i64)));
                }
                _ => {}
            }
        }

        let num_mips = match outrage_image_type {
            OutrageGraphicsFormat::OutrageTga |
            OutrageGraphicsFormat::Compressed1555Mipped | 
            OutrageGraphicsFormat::Compressed4444Mipped | 
            OutrageGraphicsFormat::CompressedMipped | 
            OutrageGraphicsFormat::CompressedNewMipped => {
                reader.read_u8().unwrap() as usize
            },
            _ => 1
        };

        trace!("mips {}", num_mips);

        let is_mipped = num_mips > 1;

        /* ignore next bytes */
        let _ = reader.seek(SeekFrom::Current(9));

        let width = reader.read_i16::<LittleEndian>().unwrap();
        let height = reader.read_i16::<LittleEndian>().unwrap();
        let pix_size = reader.read_u8().unwrap();

        trace!("width is {}", width);
        trace!("height is {}", height);
        trace!("Pix size is {}", pix_size);

        if pix_size != 32 && pix_size != 24 {
            return Err(anyhow!("pix size must be 32"));
        }

        let descriptor = reader.read_u8().unwrap();

        match descriptor & 0x0F {
            0 | 8 => {},
            _ => return Err(anyhow!("descriptor field & 0x0F must be 8 or 0"))
        }

        /* Skip over ID */
        let _ = reader.seek(SeekFrom::Current(read_image_id_length as i64));

        let mut flags = BitmapFlags::None;

        // Simulate the effects of the old bitmap system
        if is_mipped {
            flags |= BitmapFlags::MipMapped;
        }

        /* Wanting 4444 is always default */
        flags |= BitmapFlags::Wants4444;

       let data_size =
            (width as usize * height as usize) +
            num_mips as usize * ((width as usize * height as usize * 2) / 3);

        let mut bitmap = OgfBitmap {
            width: width as usize,
            height: height as usize,
            format: match (requested_format, outrage_image_type) {
                (BitmapFormat::Fmt4444, _) | (_, OutrageGraphicsFormat::Compressed4444Mipped) => {
                    BitmapFormat::Fmt4444
                },
                _ => BitmapFormat::Fmt1555
            },
            name: D3String::from(name),
            flags: flags,
            data: vec![0u16; data_size],
            mipmap_count: num_mips,
        };

        let is_upside_down = (1 - ((descriptor & 0x20) >> 5)) != 0;

        let mut pixel = 0u32;

        match outrage_image_type {
            OutrageGraphicsFormat::CompressedTga => {
                let mut total = 0;

                while total < (height * width) {
                    let command = reader.read_u8().unwrap();
                    let len = (command & 127) + 1;

                    if command & 128 != 0 {
                        if pix_size == 32 {
                            pixel = reader.read_u32::<LittleEndian>().unwrap();
                        }
                        else {
                            let r = reader.read_u8().unwrap() as u32;
                            let g =  reader.read_u8().unwrap() as u32;
                            let b =  reader.read_u8().unwrap() as u32;
                            pixel = (255 << 24) | (r << 16) | (g << 8) | b;
                        }

                        let new_pixel = tga_translate_pixel_16(pixel as i32, bitmap.format);

                        for _ in 0..len {
                            let i = total / width;
                            let t = total % width;

                            if is_upside_down {
                                let index = (((height - 1) - i) * width + t) as usize;
                                bitmap.data[index] = new_pixel;
                            }
                            else {
                                let index = (i * width + t) as usize;
                                bitmap.data[index] = new_pixel;
                            }

                            total += 1;
                        }
                    }
                }
            },
            OutrageGraphicsFormat::UncompressedTga => {
                for i in 0..height {
                    for t in 0..width {
                        if pix_size == 32 {
                            pixel = reader.read_u32::<LittleEndian>().unwrap();
                        }
                        else {
                            let r = reader.read_u8().unwrap() as u32;
                            let g =  reader.read_u8().unwrap() as u32;
                            let b =  reader.read_u8().unwrap() as u32;
                            pixel = (255 << 24) | (r << 16) | (g << 8) | b;
                        }

                        let new_pixel = tga_translate_pixel_16(pixel as i32, bitmap.format);

                        if is_upside_down {
                            let index = (((height - 1) - i) * width + t) as usize;
                            bitmap.data[index] = new_pixel;
                        }
                        else {
                            let index = (i * width + t) as usize;
                            bitmap.data[index] = new_pixel;
                        }
                    }
                }
            },
            OutrageGraphicsFormat::Compressed4444Mipped | 
            OutrageGraphicsFormat::Compressed1555Mipped | 
            OutrageGraphicsFormat::CompressedMipped | 
            OutrageGraphicsFormat::Compressed8bit | 
            OutrageGraphicsFormat::CompressedNewMipped => {
                tga_read_outrage_compressed_16(reader, &mut bitmap, num_mips, outrage_image_type);
            }
            _ => { 
                return Err(anyhow!("failed to load OGF, unknown type!"));
            }
        }

        Ok(bitmap)
    }
}

fn tga_translate_pixel_16(pixel: i32, format: BitmapFormat) -> u16 {
    let red = (pixel >> 16) & 0xFF;
    let green = (pixel >> 8) & 0xFF;
    let blue = (pixel) & 0xFF;
    let alpha = pixel >> 24 & 0xFF;
  
    if format == BitmapFormat::Fmt4444 {
        let newred = red >> 4;
        let newgreen = green >> 4;
        let newblue = blue >> 4;
        let newalpha = alpha >> 4;
        return ((newalpha << 12) | (newred << 8) | (newgreen << 4) | (newblue)) as u16;
    }
    else  {
        let newred = red >> 3;
        let newgreen = green >> 3;
        let newblue = blue >> 3;
  
        let mut newpix = OPAQUE_FLAG | (newred << 10) as u16 | (newgreen << 5) as u16 | (newblue as u16);
  
        if alpha == 0 {
            newpix = NEW_TRANSPARENT_COLOR as u16;
        }

        return newpix;
    }
}

fn tga_read_outrage_compressed_16<R: Read + Seek>(reader: &mut BufReader<R>, bitmap: &mut OgfBitmap, mipmap_count: usize, image_format: OutrageGraphicsFormat) {
    for m in 0..mipmap_count {
        let width = bitmap.get_mipmap_width(m);
        let height = bitmap.get_mipmap_height(m);
        let total = width * height;
        let mut count = 0;

        let dest_data = bitmap.get_mipped_data_slice_mut(m);

        while count != total {
            assert!(count < total);

            let command = reader.read_u8().unwrap();

            match command {
                0 => { // raw pixel
                    let mut pixel = reader.read_u16::<LittleEndian>().unwrap();

                    match image_format {
                        OutrageGraphicsFormat::Compressed1555Mipped => {},
                        OutrageGraphicsFormat::Compressed4444Mipped => {},
                        _ => {
                            if pixel == 0x07E0 {
                                pixel = NEW_TRANSPARENT_COLOR as u16;
                            }
                            else {
                                let r = ((pixel & 0xF800) >> 11) << 3;
                                let g = ((pixel & 0x07E0) >> 5) << 2;
                                let b = (pixel & 0x001F) << 3;

                                pixel = OPAQUE_FLAG | gr_rgb16!(r, g, b);
                            }
                        }
                    }

                    let i = count / width;
                    let t = count % width;

                    dest_data[i * width + t] = pixel;
                    count += 1;
                },
                c if c >= 2 && command <= 250 => {
                    // next pixel is run of pixels
                    let mut pixel = reader.read_u16::<LittleEndian>().unwrap();

                    match image_format {
                        OutrageGraphicsFormat::Compressed1555Mipped => {},
                        OutrageGraphicsFormat::Compressed4444Mipped => {},
                        _ => {
                            if pixel == 0x07E0 {
                                pixel = NEW_TRANSPARENT_COLOR as u16;
                            }
                            else {
                                let r = ((pixel & 0xF800) >> 11) << 3;
                                let g = ((pixel & 0x07E0) >> 5) << 2;
                                let b = (pixel & 0x001F) << 3;

                                pixel = OPAQUE_FLAG | gr_rgb16!(r, g, b);
                            }
                        }
                    }

                    for _ in 0..command {
                        let i = count / width;
                        let t = count % width;
                        dest_data[i * width + t] = pixel;
                        count += 1;
                    }
                },
                _ => panic!("bad compression run!")
            }
        }

        // DAJ added to fill out the mip maps down to the 1x1 size (memory is already there)
        // does not average since we are only a pixel or two in size
        if mipmap_count > 1 {
            let computed_mipmap_count = bitmap.compute_mipmap_count();
            bitmap.mipmap_count = computed_mipmap_count;

            for m in mipmap_count..computed_mipmap_count {
                let width = bitmap.get_mipmap_width(m);
                let height = bitmap.get_mipmap_width(m);

                let previous_width = bitmap.get_mipmap_width(m - 1);

                let src =  bitmap.get_mipped_data_slice(m - 1).as_ptr();
                let dst = bitmap.get_mipped_data_slice_mut(m);

                for h in 0..height {
                    for w in 0..width {
                        unsafe {
                            let offset = 2 * (h * previous_width + w) as isize;
                            dst[h * width + w] = *src.offset(offset);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::{env, fs::{File}, path::{Path, PathBuf}};
    use function_name::named;

    use crate::{display_1555, display_4444, testdata};

    use super::*;

    #[test]
    #[named]
    fn ogf_badapple_4444_1mm_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_4444_1mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt4444).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        let data_16 = bitmap.get_mipped_data_slice(0);

        // let mut bytes: Vec<u8> = Vec::with_capacity(data_16.len() * 2);
        // for &value in data_16 {
        //     let low_byte = (value & 0xFF) as u8;
        //     let high_byte = (value >> 8) as u8;
        //     bytes.push(low_byte);
        //     bytes.push(high_byte);
        // }

        // let digest = md5::compute(bytes);
        // let checksum = format!("{:x}", digest);

        display_4444!(function_name!(), data_16, bitmap.width(), bitmap.height());

        // assert_eq!(checksum, "1725cefc17154ee06a5c99e03f44004f");
    }


    #[test]
    #[named]
    fn ogf_badapple_4444_5mm_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_4444_5mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt4444).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        display_4444!(function_name!(), bitmap.get_mipped_data_slice(0), bitmap.get_mipmap_width(0), bitmap.get_mipmap_height(0));
        display_4444!(function_name!(), bitmap.get_mipped_data_slice(1), bitmap.get_mipmap_width(1), bitmap.get_mipmap_height(1));
        display_4444!(function_name!(), bitmap.get_mipped_data_slice(2), bitmap.get_mipmap_width(2), bitmap.get_mipmap_height(2));
        display_4444!(function_name!(), bitmap.get_mipped_data_slice(3), bitmap.get_mipmap_width(3), bitmap.get_mipmap_height(3));
        display_4444!(function_name!(), bitmap.get_mipped_data_slice(4), bitmap.get_mipmap_width(4), bitmap.get_mipmap_height(4));
    }


    #[test]
    #[named]
    fn ogf_badapple_1555_1mm_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_1555_1mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt1555).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        let data_16 = bitmap.get_mipped_data_slice(0);


        display_1555!(function_name!(), data_16, bitmap.width(), bitmap.height());
    }

    #[named]
    #[test]
    fn ogf_badapple_1555_5mm_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_1555_5mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt1555).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        display_1555!(function_name!(), bitmap.get_mipped_data_slice(0), bitmap.get_mipmap_width(0), bitmap.get_mipmap_height(0));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(1), bitmap.get_mipmap_width(1), bitmap.get_mipmap_height(1));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(2), bitmap.get_mipmap_width(2), bitmap.get_mipmap_height(2));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(3), bitmap.get_mipmap_width(3), bitmap.get_mipmap_height(3));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(4), bitmap.get_mipmap_width(4), bitmap.get_mipmap_height(4));
    }

    #[test]
    #[named]
    fn ogf_badapple_4444_1mm_funny_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_4444_1mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let mut bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt4444).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        bitmap.make_funny();

        let data_16 = bitmap.get_mipped_data_slice(0);

        display_4444!(function_name!(), data_16, bitmap.width(), bitmap.height());
    }

    #[named]
    #[test]
    fn ogf_badapple_1555_5mm_funny_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple_1555_5mm.ogf")).unwrap();
        let mut reader = BufReader::new(badapple);
        let mut bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt1555).unwrap();
        assert_eq!(bitmap.width(), 256);
        assert_eq!(bitmap.height(), 256);

        bitmap.make_funny();

        display_1555!(function_name!(), bitmap.get_mipped_data_slice(0), bitmap.get_mipmap_width(0), bitmap.get_mipmap_height(0));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(1), bitmap.get_mipmap_width(1), bitmap.get_mipmap_height(1));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(2), bitmap.get_mipmap_width(2), bitmap.get_mipmap_height(2));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(3), bitmap.get_mipmap_width(3), bitmap.get_mipmap_height(3));
        display_1555!(function_name!(), bitmap.get_mipped_data_slice(4), bitmap.get_mipmap_width(4), bitmap.get_mipmap_height(4));
    }
}