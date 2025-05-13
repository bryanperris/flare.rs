use std::io::{BufReader, Read, Seek};
use byteorder::{LittleEndian, ReadBytesExt, BigEndian};
use anyhow::{Context, Result};

use crate::{gr_rgb16, graphics::{bitmap, NEW_TRANSPARENT_COLOR, OPAQUE_FLAG}, string::{D3String, EMPTY}};

use super::{generate_random_color_1555, Bitmap16, BitmapFlags, BitmapFormat};

#[derive(Debug, Clone)]
pub struct PcxBitmap {
    width: usize,
    height: usize,
    data: Vec<u16>,
}

impl Bitmap16 for PcxBitmap {
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

    fn flags(&self) -> &super::BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &D3String {
        &EMPTY
    }

    fn format(&self) -> super::BitmapFormat {
        BitmapFormat::Fmt1555
    }
    
    fn make_funny(&mut self) {
        for i in 0..(self.width * self.height) {
            self.data[i] = generate_random_color_1555();
        }
    }
}

const PCX_HEADER_SIZE: usize = 128;
const HEADER_OFFSET: usize = 12;
const COLOR_INFO_OFFSET: usize = 65;
const NUM_BPP_OFFSET: usize = 3;
const VERSION_OFFSET: usize = 1;
const PLANE_SIZE_OFFSET: usize = 66;

impl PcxBitmap {
    pub fn new<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<Self> {
        let mut temp = [0u8; PCX_HEADER_SIZE];

        reader.read(&mut temp).context("Failed to read data")?;
        let _ = reader.seek(std::io::SeekFrom::Start(0));

        trace!("Plane(s): {}", temp[COLOR_INFO_OFFSET]);

        match temp[COLOR_INFO_OFFSET] {
            1 => parse_pcx_8bit(reader), // parse 8 bit
            3 => parse_pcx_24bit(reader), // parse 24-bit
            _ => Err(anyhow!("Unknown PCX depth: {}", temp[COLOR_INFO_OFFSET]))
        }
    }
}

fn parse_pcx_8bit<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<PcxBitmap> {
    let mut header = [0u8; 4];
    reader.read(&mut header).context("Failed to read header")?;

    trace!("Depth: {}", header[NUM_BPP_OFFSET]);

    if header[NUM_BPP_OFFSET] != 8 {
        return Err(anyhow!("Only 8-bit depth is acceptable"));
    }

    let xmin = reader.read_i16::<LittleEndian>().unwrap();
    let ymin = reader.read_i16::<LittleEndian>().unwrap();
    let xmax = reader.read_i16::<LittleEndian>().unwrap();
    let ymax = reader.read_i16::<LittleEndian>().unwrap();

    let mut read = [0u8; 116];
    reader.read(&mut read).context("Failed to read data")?;

    if read[COLOR_INFO_OFFSET - HEADER_OFFSET] != 1 {
        return Err(anyhow!("Must be 8 bit only"));
    }

    let width = (1 + xmax - xmin) as usize;
    let height = (1 + ymax - ymin) as usize;
    let total = width * height;

    let mut data = vec![0u8; total];
    let mut run = 0usize;

    while run < total {
        let read = reader.read_u8().unwrap();

        if read >= 192 {
            let temp = reader.read_u8().unwrap();

            for _ in 0..(read - 192) {
                data[run] = temp;
                run += 1;
            }
        }
        else {
            data[run] = read;
            run += 1;
        }
    }

    /* Ignore pad byte */
    let _ = reader.seek(std::io::SeekFrom::Current(1));

    /* Read in the palette */
    let mut p_red = [0u8; 256];
    let mut p_green = [0u8; 256];
    let mut p_blue = [0u8; 256];
    for i in 0..256 {
        p_red[i] = reader.read_u8().unwrap() >> 3;
        p_green[i] = reader.read_u8().unwrap() >> 3;
        p_blue[i] = reader.read_u8().unwrap() >> 3;
    }

    let mut bitmap = PcxBitmap {
        width: width,
        height: height,
        data: vec![0u16; total],
    };

    for i in 0..height {
        for t in 0..width {
            let c = data[i * width + t] as usize;
            let r = p_red[c] as u32;
            let g = p_green[c] as u32;
            let b = p_blue[c] as u32;

            // bitmap.data[i * width + t] = match c {
            //     0 => NEW_TRANSPARENT_COLOR as u16,
            //     _ => (OPAQUE_FLAG as u32 | (r << 10) | (g << 5) | b) as u16
            // };

            /* Let's not ignore color 0 */
            // TODO: Are there any PCXs using the specific D3 transparent colors?
            // The 24-bit version ignores transparency anyways
            bitmap.data[i * width + t] = (OPAQUE_FLAG as u32 | (r << 10) | (g << 5) | b) as u16
        }
    }

    Ok(bitmap)
}

fn parse_pcx_24bit<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<PcxBitmap> {
    let mut header = [0u8; 4];
    reader.read(&mut header).context("Failed to read header")?;

    if header[VERSION_OFFSET] != 5 {
        return Err(anyhow!("PCX Not version 5.0 or greater"));
    }

    if header[NUM_BPP_OFFSET] != 8 {
        return Err(anyhow!("Only 8bit depth is acceptabled"));
    }

    let xmin = reader.read_i16::<LittleEndian>().unwrap();
    let ymin = reader.read_i16::<LittleEndian>().unwrap();
    let xmax = reader.read_i16::<LittleEndian>().unwrap();
    let ymax = reader.read_i16::<LittleEndian>().unwrap();

    let mut read = [0u8; 116];
    reader.read(&mut read).context("Failed to read data")?;

    if read[COLOR_INFO_OFFSET - HEADER_OFFSET] != 3 {
        return Err(anyhow!("Must be 3 planes for 24bit encoding"));
    }

    let width = (1 + xmax - xmin) as usize;
    let height = (1 + ymax - ymin) as usize;

    /* Determine the bytes per line */
    let _ = reader.seek(std::io::SeekFrom::Start(PLANE_SIZE_OFFSET as u64));
    let bytes_per_line = reader.read_i16::<LittleEndian>().unwrap() as usize;
    let _ = reader.seek(std::io::SeekFrom::Start(PCX_HEADER_SIZE as u64));

    // scanline length
    let total = 3 * bytes_per_line;

    let mut data = vec![0u8; total * height];

    // Load in the data
    // the data is divided into scanlines
    // the first scanline is line 0's red
    // the second scanline is line 0's green
    // the third scanline is line 0's blue
    // the fourth scanline is line 1's red
    // etc.

    /* Red scanline */
    read_color_scanline(reader, &mut data, height, bytes_per_line);

    /* Green scanline */
    read_color_scanline(reader, &mut data, height, bytes_per_line);

    /* Blue scanline */
    read_color_scanline(reader, &mut data, height, bytes_per_line);

    let mut bitmap = PcxBitmap {
        width: width,
        height: height,
        data: vec![0u16; width * height]
    };

    for i in 0..height {
        for t in 0..width {
            let r = (data[(i * total) + (0 * bytes_per_line) + t]) as u32;
            let g = (data[(i * total) + (1 * bytes_per_line) + t]) as u32;
            let b = (data[(i * total) + (2 * bytes_per_line) + t]) as u32;

            bitmap.data[i * width + t] = OPAQUE_FLAG | gr_rgb16!(r, g, b);
        }
    }

    Ok(bitmap)
}

fn read_color_scanline<R: Read + Seek>(reader: &mut BufReader<R>, data: &mut [u8], height: usize, bytes_per_line: usize) {
    let mut offset = 0;

    for line in 0..height {
        let mut run = 0;

        while run < bytes_per_line {
            let read = reader.read_u8().unwrap();

            if read >= 192 {
                let temp = reader.read_u8().unwrap();

                for _ in 0..(read - 192) {
                    data[offset] = temp;
                    run += 1;
                    offset += 1;
                }
            }
            else {
                data[offset] = read;
                run += 1;
                offset += 1;
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
    fn ogf_badapple_pcx_test() {
        crate::test_common::setup();

        let badapple = File::open(testdata!("badapple.pcx")).unwrap();
        let mut reader = BufReader::new(badapple);
        let bitmap = PcxBitmap::new(&mut reader).unwrap();
        assert_eq!(bitmap.width(), 480);
        assert_eq!(bitmap.height(), 360);

        display_1555!(function_name!(), &bitmap.data, bitmap.width(), bitmap.height());
    }
}