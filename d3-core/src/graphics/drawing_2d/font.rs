
use core::{borrow::{Borrow, BorrowMut}, cell::RefCell, default, ops::Range, ptr::read};
use std::{io::{BufReader, BufWriter, Cursor, Read, Seek}, rc::Rc};
use crate::{common::unsigned_safe_sub, graphics::{ddgr_color, drawing_2d::font, rendering::Renderer}, string::D3String};

use crate::{gr_color_to_16, gr_rgb, gr_rgb16, graphics::{bitmap::{Bitmap16, BitmapFlags, BitmapFormat}, BitsPerPixelType, NEW_TRANSPARENT_COLOR, OPAQUE_FLAG, OPAQUE_FLAG16}};

use anyhow::{Context, Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, BigEndian};

use bitflags::bitflags;

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct FontFlags: u16 {
        const None  =             0b00000000;
        const Color =             0b00000001;
        const Proportional =      0b00000010;
        const Kerned       =      0b00000100;
        const Gradient     =      0b00001000;
        const Fmt4444      =      0b00010000;
        const FFi2         =      0b00100000;
        const UnknownFlag  =      0b01000000;
        const Uppercase    =      0b10000000;
    }
}

macro_rules! bits_to_bytes {
    ($v:expr) => {
        ($v + 7) >> 3
    };
}

macro_rules! bites_to_shorts {
    ($v:expr) => {
        ($v + 15) >> 4
    };
}


pub struct Font2 {
    pub tracking: i16,
    pub reserved: [u8; 62],
}

impl Default for Font2 {
    fn default() -> Self {
        Self { 
            tracking: Default::default(), 
            reserved: [0u8; 62] 
        }
    }
}

pub struct Font {
    name: String,
    /// width of widest character and height of longest char
    width: usize,
    height: usize,
    /// flags used by the character renderer
    flags: FontFlags,
    /// pixels given to lowercase below script line start at baseline
    baseline: i16,
    /// minimum ascii value used by font
    min_ascii: usize,
    /// max ascii value used by the font
    max_ascii: usize,
    /// width of a character in the font in bytes
    byte_width: i16,
    /// pixel, map data.
    raw_data: Vec<u8>,
    /// pointers to each character
    char_data: Vec<Range<usize>>,
    /// individual pixel widths of each character
    char_widths: Option<Vec<usize>>,
    /// Kerning information for specific letter combos
    kern_data: Option<Vec<u8>>,
    /// FFI2 (newstyle) data
    ffi2: Option<Font2>,
    /// this IS NOT in the file, but a part of the baseline element. (upper 8bits)
    brightness: f32,
}

fn ascii_toupper(c: usize) -> usize {
    if c >= b'a'.into() && c <= b'z'.into() {
        c - 32
    } else {
        c
    }
}

pub struct FontTemplate {
    min_ascii: usize,
    max_ascii: usize,
    character_widths: Option<Vec<usize>>,
    kern_data: Option<Vec<u8>>,
    character_height: usize,
    character_max_width: usize,
    //// is this font proportional? if so use array of widths, else use maxwidth
    is_proportional: bool,
    /// uppercase font?
    is_uppercase: bool,
    /// font is monochromatic?
    is_monochromatic: bool,
    /// new style 4444 font.
    is_newstyle: bool,
    /// new font info added.
    is_ffi2: bool,
    /// ffi2 style (font file info 2)
    character_trackng: i8
}

impl FontTemplate {
    fn resolve_char_index(&self, index:usize) -> usize {
        if self.min_ascii > index || self.max_ascii < index {
            panic!("invalid char range for D3 font: char code: {}, max {}, min {}", index, self.max_ascii, self.min_ascii);
        }

        index - self.min_ascii
    }

    pub fn character_width(&self, ascii_character: usize) -> usize {
        let mut ch = self.resolve_char_index(ascii_character);

        if ascii_character > self.max_ascii && self.is_uppercase {
            ch = ascii_toupper(ch);
        }
        else if (ch < self.min_ascii || ch > self.max_ascii) {
            return 0;
        }

        if self.is_proportional {
            self.character_widths.as_ref().unwrap()[(ch - self.min_ascii) as usize]
        }
        else {
            self.character_max_width
        }
    }

    pub fn character_height(&self) -> usize {
        self.character_height
    }

    pub fn character_spacing(&self, ch1: usize, ch2: usize) -> usize {
        let new_ch1 = 
            if ch1 as usize > self.max_ascii && self.is_uppercase 
            { crate::string_common::to_uppercase_ascii(ch1 as i32) as usize }
            else { ch1 };

        let new_ch2 = 
            if ch2 as usize > self.max_ascii && self.is_uppercase 
            { crate::string_common::to_uppercase_ascii(ch2 as i32) as usize }
            else { ch2 };

        return self.get_kerned_spacing(new_ch1, new_ch2)
    }

    pub fn get_kerned_spacing(&self, ch1: usize, ch2: usize) -> usize {
        let kern_data = self.kern_data.as_ref().unwrap().as_slice();
        let mut offset = 0;

        while kern_data[offset] != 255 {
            if ch1 == kern_data[offset] as usize && ch2 == kern_data[offset + 1] as usize {
                return kern_data[offset + 2] as i8 as usize;
            }
            offset += 3;
        }

        0
    }
}

impl Default for Font {
    fn default() -> Self {
        Self { 
            name: Default::default(), 
            width: Default::default(), 
            height: Default::default(), 
            flags: FontFlags::None, 
            baseline: Default::default(), 
            min_ascii: Default::default(), 
            max_ascii: Default::default(), 
            byte_width: Default::default(), 
            raw_data: Default::default(), 
            char_data: Default::default(), 
            char_widths: None, 
            kern_data: Default::default(), 
            ffi2: Default::default(), 
            brightness: Default::default() 
        }
    }
}

impl Font {
    fn resolve_char_index(&self, index:usize) -> usize {
        if self.min_ascii > index || self.max_ascii < index {
            panic!("invalid char range for D3 font: char code: {}, max {}, min {}", index, self.max_ascii, self.min_ascii);
        }

        index - self.min_ascii
    }

    pub(crate) fn get_raw_char_data(&self, raw_index: usize) -> &[u8] {
        &self.raw_data[self.char_data[raw_index].clone()]
    }

    pub fn get_char_data(&self, index: usize) -> &[u8] {
        &self.raw_data[self.char_data[self.resolve_char_index(index)].clone()]
    }

    pub fn get_char_width(&self, index: usize) -> usize {
        if self.flags.contains(FontFlags::Proportional) {
            self.char_widths.as_ref().unwrap()[self.resolve_char_index(index)]
        }
        else {
            self.width
        }
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_ascii_range(&self) -> Range<usize> {
        self.min_ascii .. self.max_ascii
    }

    fn toggle_flag(&mut self, condition: bool, flags: FontFlags) {
        if condition {
            self.flags.insert(flags);
        }
        else {
            self.flags.remove(flags);
        }
    }

    pub fn new_from_steam<R: Read + Seek>(name: String, reader: &mut BufReader<R>) -> Result<Self> {
        let mut font = Font::default();

        /* verify the ID */
        let id = reader.read_u32::<LittleEndian>().unwrap();

        if id != 0xFEEDBABA {
            return Err(anyhow!("magic in font not valid"));
        }

        font.name = name;
        font.width = reader.read_u16::<LittleEndian>().unwrap() as usize;
        font.height = reader.read_u16::<LittleEndian>().unwrap() as usize;
        font.flags = FontFlags::from_bits(reader.read_u16::<LittleEndian>().unwrap()).unwrap();
        font.baseline = reader.read_u16::<LittleEndian>().unwrap() as i16;
        font.min_ascii = reader.read_u8().unwrap() as usize;
        font.max_ascii = reader.read_u8().unwrap() as usize;

        /* Skip over embedded font name */
        let _ = reader.seek(std::io::SeekFrom::Current(32));

        if font.flags.contains(FontFlags::FFi2) {
            let mut ffi2 = Font2::default();
            ffi2.tracking = reader.read_i16::<LittleEndian>().unwrap();
            reader.read_exact(&mut ffi2.reserved).context("Failed to read reserved")?;
            font.ffi2 = Some(ffi2);
        }

        font.brightness = (font.baseline >> 8) as f32 / 10.0f32;

        let num_chars = font.max_ascii - font.min_ascii + 1;

        if (font.max_ascii as u8 as char) < 'a' {
            font.flags |= FontFlags::Uppercase;
        }

        if font.flags.contains(FontFlags::Proportional) {
            let mut widths = vec![0usize; num_chars as usize];

            for w in &mut widths {
                *w = reader.read_i16::<LittleEndian>().unwrap() as u8 as usize;
            }

            font.char_widths = Some(widths);
        }

        // TODO: Read in kerning data
        if font.flags.contains(FontFlags::Kerned) {
            let num_pairs = reader.read_u16::<LittleEndian>().unwrap() as usize;

            let mut kern_data = vec![0u8; 3 * (num_pairs + 1)];

            for i in 0..num_pairs {
                kern_data[i * 3 + 0] = reader.read_u8().unwrap();
                kern_data[i * 3 + 1] = reader.read_u8().unwrap();
                kern_data[i * 3 + 2] = reader.read_u8().unwrap();
            }

            kern_data[num_pairs * 3] = 255;
            kern_data[num_pairs * 3 + 1] = 255;
            kern_data[num_pairs * 3 + 2] = 0;

            font.kern_data = Some(kern_data);
        }

        //	Read in pixel data.
        //	for color fonts, read in byte count and then the data,
        //		generate character data pointer table
        //	for mono fonts, read in byte count, then the data, convert to bits and store
        //		generate character data pointer table

        let byte_size = reader.read_u32::<LittleEndian>().unwrap() as usize;
        let mut raw_data = vec![0u8; byte_size];
        let mut char_data: Vec<Range<usize>> = Vec::default();

        reader.read_exact(&mut raw_data).context("Failed to read raw_data")?;
        font.raw_data = raw_data;


        if font.flags.contains(FontFlags::Color) {
            let mut offset = 0;

            for i in 0..num_chars as usize {
                let size = if font.flags.contains(FontFlags::Proportional) {
                    font.char_widths.as_ref().unwrap()[i] * font.height * bits_to_bytes!(BitsPerPixelType::Bbp16 as usize)
                }
                else {
                    font.width * font.height * bits_to_bytes!(BitsPerPixelType::Bbp16 as usize)
                };
                
                char_data.push(offset .. offset + size);
                offset += size;
            }
        }
        else {
            let mut offset = 0;

            for i in 0..num_chars as usize {
                let size = if font.flags.contains(FontFlags::Proportional) {
                    bits_to_bytes!(font.char_widths.as_ref().unwrap()[i]) * font.height
                }
                else {
                    bits_to_bytes!(font.width) * font.height
                };
                

                char_data.push(offset .. offset + size);
                offset += size;
            }
        }

        font.char_data = char_data;
  
        Ok(font)
    }


    pub fn apply_template(font: &mut Font, template: &FontTemplate) {
        font.width = template.character_max_width;
        font.height = template.character_height;

        font.toggle_flag(template.is_proportional, FontFlags::Proportional);
        font.toggle_flag(!template.is_monochromatic, FontFlags::Color);
        font.toggle_flag(template.kern_data.is_some(), FontFlags::Kerned);
        font.toggle_flag(template.is_newstyle, FontFlags::Fmt4444);

        if template.is_ffi2 {
            font.toggle_flag(true, FontFlags::FFi2);
        }

        font.kern_data = template.kern_data.clone();
        font.char_widths = template.character_widths.clone();
    }

    /// returns the raw bitmap data for a character in a font, its width and height
    /// returned data should be in 565 hicolor format if is_mono is false.  if is_mono is true,
    ///	then a bitmask will be returned, and you should treat a bit as a pixel.
    pub fn get_raw_char_bitmap(&self, index: usize) -> Option<(usize, usize, bool, Box<[u16]>)> {
        let mut ch = if index > self.max_ascii && self.flags.contains(FontFlags::Uppercase) {
            ascii_toupper(index)
        }
        else {
            index
        };

        if ch < self.min_ascii || ch > self.max_ascii {
            return None;
        }

        ch -= self.min_ascii as usize;

        let is_mono = !self.flags.contains(FontFlags::Color);
        let w = self.get_char_width(ch);
        let h = self.height;

        let mut data = vec![0u16; w * h];
        let mut cursor = Cursor::new(self.get_raw_char_data(ch));
        let mut reader = BufReader::new(cursor);

        for _ in 0..&self.char_data[ch].len() / 2 {
            data.push(reader.read_u16::<LittleEndian>().unwrap());
        }

        Some((w, h, is_mono, data.into_boxed_slice()))
    }

    pub fn get_kerned_spacing(&self, ch1: usize, ch2: usize) -> isize {
        let ch1 = self.resolve_char_index(ch1);
        let ch2 = self.resolve_char_index(ch2);

        if self.kern_data.is_some() {
            let kern_data = self.kern_data.as_ref().unwrap().as_slice();
            let mut offset = 0;

            while kern_data[offset] != 255 {
                if ch1 == kern_data[offset] as usize && ch2 == kern_data[offset + 1] as usize {
                    return kern_data[offset + 2] as i8 as isize;
                }
                offset += 3;
            }
        }

        0
    }

    pub fn get_tracking(&self) -> usize {
        self.ffi2.as_ref().unwrap().tracking as usize
    }
}


pub struct FontGraphic {
    char_bitmaps: Vec<Rc<FontBitmap16>>,
    char_info: Vec<CharBitmapInfo>,
    font: Rc<Font>
}

#[derive(Debug, Copy, Clone)]
struct CharBitmapInfo {
    bitmap_index: usize,
    tex_u: usize,
    tex_v: usize
}

#[derive(Clone)]
pub struct CharBitmapTexSrc {
    pub bitmap_src: Rc<dyn Bitmap16>,
    pub u: usize,
    pub v: usize,
    pub width: usize,
    pub height: usize
}

impl FontGraphic {
    const FONT_SURFACE_WIDTH: usize = 128;
    const FONT_SURFACE_HEIGHT: usize = 128;

    pub fn new(font: Font) -> Rc<Self> {
        let font_rc = Rc::new(font);

        let mut fg = FontGraphic {
            font: Rc::clone(&font_rc),
            char_info: Vec::default(),
            char_bitmaps: Vec::default()
        };
        

        fg.generate_char_bitmap16s(&font_rc).unwrap();

        Rc::new(fg)
    }

    pub fn get_height(&self) -> usize {
        self.font.get_height()
    }

    pub(crate) fn get_bitmaps(&self) -> &[Rc<FontBitmap16>] {
        self.char_bitmaps.as_slice()
    }

    fn resolve_ascii_range(&self, index: usize) -> usize {
        if self.font.min_ascii > index || self.font.max_ascii < index {
            panic!("char out of supported ASCII range");
        }

        index - self.font.min_ascii
    }

    pub fn get_char_tex_source(&self, index: usize) -> CharBitmapTexSrc {
        let char_index = self.resolve_ascii_range(index);

        CharBitmapTexSrc {
            bitmap_src: self.char_bitmaps[self.char_info[char_index].bitmap_index].clone(),
            u: self.char_info[char_index].tex_u,
            v: self.char_info[char_index].tex_v,
            width: self.font.get_char_width(index),
            height: self.font.get_height()
        }
    }

    pub fn clone_char_bitmap(&self, index: usize) -> (Box<[u16]>, usize, usize) {
        let char_tex_source = self.get_char_tex_source(index);
        let mut char_bitmap = vec![0u16; char_tex_source.width * char_tex_source.height];

        for y in 0..char_tex_source.height {
            for x in 0..char_tex_source.width {
                let src = (char_tex_source.v + y) * Self::FONT_SURFACE_WIDTH + (char_tex_source.u + x);
                let dst = y * char_tex_source.width + x;
                char_bitmap[dst] = char_tex_source.bitmap_src.data()[src];
            }
        }

        (char_bitmap.into_boxed_slice(), char_tex_source.width, char_tex_source.height)
    }

    pub fn get_font(&self) -> &Rc<Font> {
        &self.font
    }

    pub fn get_char_width(&self, index: usize) -> usize {
        if self.font.flags.contains(FontFlags::Proportional) {
            self.font.char_widths.as_ref().unwrap()[index]
        }
        else {
            self.font.width
        }
    }

    pub fn get_char_normalized_texcoords(&self, index: usize) -> (f32, f32, f32, f32) {
        let char_index = self.resolve_ascii_range(index);

        (
            self.char_info[char_index].tex_u as f32 / Self::FONT_SURFACE_WIDTH as f32,
            self.char_info[char_index].tex_v as f32 / Self::FONT_SURFACE_HEIGHT as f32,
            self.font.get_char_width(index) as f32 / Self::FONT_SURFACE_WIDTH as f32,
            self.font.get_height() as f32 / Self::FONT_SURFACE_HEIGHT as f32
        )
    }

    fn generate_char_bitmap16s(&mut self, font: &Rc<Font>) -> Result<()> {
        let mut u = 0;
        let mut v = 0;

        let mut bitmap;

        // Generate the surface bitmaps

        bitmap = Rc::new(FontBitmap16 {
            data: vec![NEW_TRANSPARENT_COLOR as u16; Self::FONT_SURFACE_WIDTH as usize * Self::FONT_SURFACE_HEIGHT],
            format: if self.font.flags.contains(FontFlags::Fmt4444) {
                BitmapFormat::Fmt4444
            }
            else {
                BitmapFormat::Fmt1555
            }
        });

        for i in 0..self.font.char_data.len() {
            let w = self.get_char_width(i);

            if (u + w) > Self::FONT_SURFACE_WIDTH {
                u = 0;
                v += self.font.get_height();
    
                if v + self.font.get_height() > Self::FONT_SURFACE_HEIGHT {

                    /* Always push the last instance */

                    self.char_bitmaps.push(Rc::clone(&bitmap));

                    bitmap = Rc::new(FontBitmap16 {
                        data: vec![NEW_TRANSPARENT_COLOR as u16; Self::FONT_SURFACE_WIDTH * Self::FONT_SURFACE_HEIGHT],
                        format: if self.font.flags.contains(FontFlags::Fmt4444) {
                            BitmapFormat::Fmt4444
                        }
                        else {
                            BitmapFormat::Fmt1555
                        }
                    });

                    v = 0;
                }
            }

            // Blit the character bitmap
            if self.font.flags.contains(FontFlags::Color) {
                if self.font.flags.contains(FontFlags::Gradient) {
                    translate_color_gray_char(Rc::get_mut(&mut bitmap).unwrap(), &self.font, u, v, i, w);
                }

                translate_color_char(Rc::get_mut(&mut bitmap).unwrap(), &self.font, u, v, i, w);
            }
            else {
                translate_mono_char(Rc::get_mut(&mut bitmap).unwrap(), &self.font, u, v, i, w);
            }
        
            // #[cfg(test)]
            // {
            //     let mut new_buffer = vec![0u16; w * self.font.get_height()];

            //     for y in 0..self.font.get_height() {
            //         for x in 0..w {
            //             let old_index = (v + y) * 128 + (u + x);
            //             let new_index = y * w + x;
            //             new_buffer[new_index] = bitmap.data()[old_index];
            //         }
            //     }

            //     crate::display_4444!("asdadasda", &new_buffer, w, self.font.get_height());

            //     trace!("tex u: {}, v: {} for {}", u, v, (i + self.font.min_ascii) as u8 as char );
            // }

            self.char_info.push(CharBitmapInfo {
                bitmap_index: self.char_bitmaps.len(),
                tex_u: u,
                tex_v: v
            });

            u += w;
        }

        /* Ensure the last one gets pushed too */
        self.char_bitmaps.push(Rc::clone(&bitmap));

        trace!("Font to bitmap mapping: {} bimaps generated(s)", self.char_bitmaps.len());

        Ok(())
    }
}


#[derive(Debug, Clone)]
pub(crate) struct FontBitmap16 {
    format: BitmapFormat,
    data: Vec<u16>,
}

impl Bitmap16 for FontBitmap16 {
    fn data(&self) -> &[u16] {
        &self.data
    }

    fn width(&self) -> usize {
        FontGraphic::FONT_SURFACE_WIDTH
    }

    fn height(&self) -> usize {
        FontGraphic::FONT_SURFACE_HEIGHT
    }

    fn mip_levels(&self) -> usize {
        0
    }

    fn flags(&self) -> &crate::graphics::bitmap::BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &D3String {
        todo!("name not given for font bitmaps")
    }

    fn format(&self) -> crate::graphics::bitmap::BitmapFormat {
        self.format
    }

    fn make_funny(&mut self) {
        match self.format() {
            BitmapFormat::Fmt4444 => {
                for i in 0..self.width() * 2 {
                    self.data[i] = crate::graphics::bitmap::generate_random_color_4444();
                }
            },
            BitmapFormat::Fmt1555 => {
                for i in 0..self.height() * 2 {
                    self.data[i] = crate::graphics::bitmap::generate_random_color_1555();
                }
            },
            _ => {}
        }
    }
}

pub struct FontCache {
    fonts: Rc<FontGraphic>,

}

impl FontCache {
    pub fn load_font<R: Read + Seek>(reader: &mut BufReader<R>) {

    }
}

fn translate_mono_char(bitmap: &mut FontBitmap16, font: &Font, x: usize, y: usize, index: usize, width: usize) {
    let color_white = gr_color_to_16!(gr_rgb!(255, 255, 255));
    let rowsize_w = bitmap.width();

    let mut data_offset = y * rowsize_w;
    let mut font_data_offset = 0;
    let mut font_read = 0u8;

    for _ in 0..font.height {
        let mut bit_mask = 0;

        for col in 0..width {
            if bit_mask == 0 {
                font_read = font.get_raw_char_data(index)[font_data_offset];
                font_data_offset += 1;
                bit_mask = 0x80;
            }

            if font_read & bit_mask != 0 {
                bitmap.data[data_offset + col] = color_white | OPAQUE_FLAG;
            }

            bit_mask >>= 1;
        }
        data_offset += rowsize_w;
    }

}

fn translate_color_char(bitmap: &mut FontBitmap16, font: &Font, x: usize, y: usize, index: usize, width: usize) {
    /*	16-bit copy from source bitmap to destination surface just created and
        locked
        This function performs scaling if the source width and height don't match
        that of the destinations - JL
    */

    let mut src = Cursor::new(font.get_raw_char_data(index));
    let mut reader = BufReader::new(src);
    let rowsize_w = bitmap.width();

    let mut dst_offset = y * rowsize_w;

    if font.flags.contains(FontFlags::Fmt4444) {
        for _ in 0..font.height {
            for col in 0..width {
                bitmap.data[dst_offset + x + col] = reader.read_u16::<LittleEndian>().unwrap();
            }
            dst_offset += rowsize_w;
        }
    }
    else {
        for _ in 0..font.height {
            for col in 0..width {
                let col_565 =  reader.read_u16::<LittleEndian>().unwrap();

                if col_565 == 0x07E0 {
                    bitmap.data[dst_offset + x + col] = NEW_TRANSPARENT_COLOR as u16;
                }
                else {
                    bitmap.data[dst_offset + x + col] =
                        ((col_565 & 0xF800) >> 1) |
                        ((col_565 & 0x07C0) >> 1) |
                        (col_565 & 0x001F) |
                        OPAQUE_FLAG;
                }
            }
            dst_offset += rowsize_w;
        }
    }
}

fn translate_color_gray_char(bitmap: &mut FontBitmap16, font: &Font, x: usize, y: usize, index: usize, width: usize) {
    /*	16-bit copy from source bitmap to destination surface just created and
        locked
        This function performs scaling if the source width and height don't match
        that of the destinations - JL
    */

    let mut src = Cursor::new(font.get_raw_char_data(index));
    let mut reader = BufReader::new(src);
    let rowsize_w = bitmap.width();

    let mut dst_offset = y * rowsize_w;

    let recip = 1.0 / 32.0;

    for _ in 0..font.height {
        for col in 0..width {
            let color_565 = reader.read_u16::<LittleEndian>().unwrap();

            if color_565 == 0x07E0 {
                bitmap.data[dst_offset + x + col] = NEW_TRANSPARENT_COLOR as u16;
            }
            else {
                let r = ((color_565 & 0xF800) >> 11) as u8;
                let g = ((color_565 & 0x07C0) >> 6) as u8;
                let b = (color_565 & 0x001F) as u8;

                let brightness = 
                    (r as f32 * 0.30f32) +
                    (g as f32 * 0.59f32) +
                    (b as f32 * 0.11f32) *
                    recip;

                let elem = if (brightness * font.brightness) > 1.0 {
                    255.0
                }
                else {
                    255.0 * brightness * font.brightness
                };

                bitmap.data[dst_offset + x + col] = gr_rgb16!(elem as u16, elem as u16, elem as u16) | OPAQUE_FLAG16;
            }
        }
        dst_offset += rowsize_w;
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphClipRect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphDrawRect {
    pub x1: usize,
    pub y1: usize,
    pub x2: usize,
    pub y2: usize,

    // Normalized units for GPU
    pub u: f32,
    pub v: f32,
    pub w: f32,
    pub h: f32
}

impl Default for GlyphDrawRect {
    fn default() -> Self {
        Self { 
            x1: Default::default(), 
            y1: Default::default(), 
            x2: Default::default(), 
            y2: Default::default(), 
            u: Default::default(), 
            v: Default::default(), 
            w: Default::default(), 
            h: Default::default() 
        }
    } 
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct FontGlyph {
    pub character_index: usize,
    pub x: usize,
    pub y: usize,
    pub scale_x: f32,
    pub scale_y: f32,
    pub clip: Option<GlyphClipRect>,
    pub draw_rect: GlyphDrawRect
}

impl Default for FontGlyph {
    fn default() -> Self {
        Self { 
            character_index: 0, 
            x: 0, 
            y: 0, 
            scale_x: 1.0, 
            scale_y: 1.0, 
            clip: None,
            draw_rect: Default::default()
        }
    }
}

impl FontGlyph {
    pub fn compute_drawing_rect(&mut self, font_graphic: &FontGraphic) -> usize {
        let font = &font_graphic.font;

        // We compute the clipping bounds 

        if self.character_index > font.max_ascii && font.flags.contains(FontFlags::Uppercase) {
            self.character_index = ascii_toupper(self.character_index);
        }

        if self.character_index < font.min_ascii || self.character_index > font.max_ascii {
            return self.x + 1;
        }

        // Lets not do this, we should retain the original char index
        // When the index is passed to the font functions
        // They deal with the conversion
        // self.character_index -= font.min_ascii;

        if self.clip.is_none() {
            let clip = GlyphClipRect {
                w: font.get_char_width(self.character_index),
                h: font.get_height(),
                x: 0,
                y: 0
            };

            self.clip = Some(clip);

            let fg_rect = font_graphic.get_char_normalized_texcoords(self.character_index);

            self.draw_rect = GlyphDrawRect {
                x1: self.x,
                y1: self.y,
                x2: self.x + clip.w * self.scale_x.trunc() as usize,
                y2: self.y + clip.h * self.scale_y.trunc() as usize,
                u: fg_rect.0,
                v: fg_rect.1,
                w: fg_rect.2,
                h: fg_rect.3
            };

            return self.x + (self.clip.as_ref().unwrap().w * self.scale_x.trunc() as usize);
        }
        else {
            // Values will already be scaled

            let clip = self.clip.as_ref().unwrap();

            let fg_rect = font_graphic.get_char_normalized_texcoords(self.character_index);

            self.draw_rect = GlyphDrawRect {
                x1: self.x,
                y1: self.y,
                x2: self.x + clip.w,
                y2: self.y + clip.h,
                u: fg_rect.0 + (clip.x as f32 / FontGraphic::FONT_SURFACE_WIDTH as f32),
                v: fg_rect.1 + (clip.y as f32 / FontGraphic::FONT_SURFACE_HEIGHT as f32),
                w: fg_rect.2 / FontGraphic::FONT_SURFACE_WIDTH as f32,
                h: fg_rect.3 / FontGraphic::FONT_SURFACE_HEIGHT as f32
            };

            return self.x + clip.w;
        }
    }
}

// TODO: void grfont_Spew(int font, int x, int y)
// TODO: int grfont_KeyToAscii(int font, int key)
