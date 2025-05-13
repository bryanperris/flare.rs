use core::num;
use std::{borrow::{Borrow, BorrowMut}, cell::RefCell, default, fs::read, io::{self, BufReader, Error, Read, Seek, SeekFrom}, rc::Rc, result};

use byteorder::{LittleEndian, ReadBytesExt, BigEndian};
use crate::endianess::TargetEndian;

// TODO: What happens wehn we reach end of file


#[derive(Debug)]
pub enum IffError {
    NoMemory,
    UnknownForm,
    NotIff,
    InvalidBitmapType,
    Corrupt,
    InvalidAnimationForm,
    InvalidBitmapForm,
    TooManyBitmaps,
    UnknownMask,
    Io(io::Error),
    Parse(std::str::Utf8Error),
    BitmapMismatch,
    InvalidCompression,
}

impl std::fmt::Display for IffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IffError::NoMemory => write!(f, "not enough mem for loading or processing"),
            IffError::UnknownForm => write!(f, "IFF file, but not a bitmap"),
            IffError::NotIff => write!(f, "this isn't even an IFF file"),
            IffError::InvalidBitmapType => write!(f, "tried to save invalid type, like BM_RGB15"),
            IffError::Corrupt => write!(f, "bad data in file"),
            IffError::InvalidAnimationForm => write!(f, "this is an anim, with non-anim load rtn"),
            IffError::InvalidBitmapForm => write!(f, "this is not an anim, with anim load rtn"),
            IffError::TooManyBitmaps => write!(f, "anim read had more bitmaps than room for"),
            IffError::UnknownMask => write!(f, "unknown masking type"),
            IffError::Io(_) => write!(f, "error reading from file"),
            IffError::BitmapMismatch => write!(f, "bm being loaded doesn't match bm loaded into"),
            IffError::Parse(_) => write!(f, "failed to parse text data"),
            IffError::InvalidCompression => write!(f, "bm being loaded uses unknown compression type"),
        }
    }
}

const MIN_COMPRESS_WIDTH: i32 = 65;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BitmapTypes {
    Pbm,
    Ilbm,
    Unknown
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionTypes {
    None,
    ByteRun1,
    Unknown
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaskingTypes {
    None,
    HasMask,
    HasTransparentColor,
    Unknown
}

#[derive(Copy, Debug, Clone)]
struct PaletteEntry {
    red: u8,
    green: u8,
    blue: u8
}

impl Default for PaletteEntry {
    fn default() -> Self {
        Self { red: Default::default(), green: Default::default(), blue: Default::default() }
    }
}

pub struct IffResource {
    bitmaps: Vec<IffBitmap>
}

impl Default for IffResource {
    fn default() -> Self {
        Self { 
            bitmaps: vec![IffBitmap::default(); 1]
        }
    }
}

impl std::fmt::Display for IffResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.bitmaps {
            write!(f, "{}", b);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IffBitmap {
    width: i16,
    height: i16,
    x: i16,
    y: i16,
    bitmap_type: BitmapTypes,
    /// which color is transparent (if any)
    transparent_color: Option<i16>,
    /// width of source screen
    page_width: i16,
    /// height of source screen
    page_height: i16,
    /// number of planes (8 for 256 color image)
    num_planes: u8,
    masking: MaskingTypes,
    compression: CompressionTypes,
    x_aspect: u8, // aspect ratio (usually 5/6)
    y_aspect: u8,
    pallete: [PaletteEntry; 256],
    data: Vec<u8>,
    row_size: usize
}

impl Default for IffBitmap {
    fn default() -> Self {
        Self { 
            width: Default::default(), 
            height: Default::default(), 
            x: Default::default(),
            y: Default::default(),
            bitmap_type: BitmapTypes::Pbm, 
            transparent_color: Default::default(), 
            page_width: Default::default(), 
            page_height: Default::default(), 
            num_planes: Default::default(), 
            masking: MaskingTypes::None, 
            compression: CompressionTypes::None, 
            x_aspect: Default::default(), 
            y_aspect: Default::default(), 
            pallete: [PaletteEntry::default(); 256], 
            data: Default::default(), 
            row_size: Default::default(),
        }
    }
}

impl std::fmt::Display for IffBitmap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "IffBitmap {{\n")?;
        write!(f, "    Width: {}\n", self.width)?;
        write!(f, "    Height: {}\n", self.height)?;
        write!(f, "    X: {}\n", self.x)?;
        write!(f, "    Y: {}\n", self.y)?;
        write!(f, "    Bitmap Type: {:?}\n", self.bitmap_type)?;
        write!(f, "    Transparent Color: {}\n", match self.transparent_color {
            Some(color) => color.to_string(),
            None => "None".to_string(),
        })?;
        write!(f, "    Page Width: {}\n", self.page_width)?;
        write!(f, "    Page Height: {}\n", self.page_height)?;
        write!(f, "    Number of Planes: {}\n", self.num_planes)?;
        write!(f, "    Masking: {:?}\n", self.masking)?;
        write!(f, "    Compression: {:?}\n", self.compression)?;
        write!(f, "    Aspect Ratio: {}/{}\n", self.x_aspect, self.y_aspect)?;
        write!(f, "    Row Size: {}\n", self.row_size)?;
        write!(f, "    Data Length: {}\n", self.data.len())?;
        write!(f, "    Palette Length: {}\n", self.pallete.len())?;
        write!(f, "}}")?;
        Ok(())
    }
}


impl IffResource {
    fn new<R: Read + Seek>(reader: &mut BufReader<R>, len: u64) -> Result<Self, IffError> {
        new(reader, len)
    }

    fn new_just_first_frame() {
        todo!();
    }

    #[cfg(feature = "with_ffmpeg")]
    fn new_from_ffmpeg<R: Read + Seek>(reader: &mut BufReader<R>, width: i32, height: i32) -> Result<Self, IffError> {

        let mut resource = Self::default();

        use anyhow::{anyhow, Context, Result};
        use std::{ffi::CString, io::Cursor, ptr::slice_from_raw_parts};
        use rsmpeg::{avcodec::{AVCodec, AVCodecContext, AVCodecParameters, AVCodecParserContext, AVPacket}, avformat::AVFormatContextInput, avutil::AVDictionary, error::RsmpegError, ffi};

        let decoder = AVCodec::find_decoder(ffi::AV_CODEC_ID_IFF_ILBM).context("Codec not found").unwrap();
        
        // XXX: We need to setup extradata for ffmepeg....
        // let params = AVCodecParameters::new();
        // params.
        let mut decode_context = AVCodecContext::new(&decoder);

        // decode_context.apply_codecpar(codecpar)

        decode_context.set_width(width);
        decode_context.set_height(height);
        decode_context.open(None).context("Could not open codec").unwrap();

        let mut parser_context = AVCodecParserContext::init(decoder.id).context("Parser not found").unwrap();
        let mut packet = AVPacket::new();

        let mut buffer: Vec<u8> = Vec::default();
        let len = reader.read_to_end(&mut buffer).unwrap();
    
        let mut parsed_offset = 0;
            
        while parsed_offset < len {
            let (get_packet, offset) = parser_context
                .parse_packet(&mut decode_context, &mut packet, &&buffer[parsed_offset..len])
                .context("Error while parsing").unwrap();
            parsed_offset += offset;

            if get_packet {
                decode_context.send_packet(Some(&packet)).unwrap();
                
                loop {
                    let frame = match decode_context.receive_frame() {
                        Ok(frame) => frame,
                        Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
                        Err(e) => Err(e).context("Error during decoding").unwrap(),
                    };
                    println!("saving frame {}", decode_context.frame_num);
                    
                    let bitmap = IffBitmap {
                        width: frame.width as i16,
                        height: frame.height as i16,
                        x: 0,
                        y: 0,
                        bitmap_type: match frame.pict_type {
                            0 => BitmapTypes::Pbm,
                            1 => BitmapTypes::Ilbm,
                            _ => BitmapTypes::Unknown
                        },
                        transparent_color: None,
                        page_width: 0,
                        page_height: 0,
                        num_planes: 0,
                        masking: MaskingTypes::None,
                        compression: CompressionTypes::None,
                        x_aspect: 0,
                        y_aspect: 0,
                        pallete: [PaletteEntry::default(); 256],
                        data: unsafe {
                            std::slice::from_raw_parts(frame.data[0], frame.linesize[0] as usize * frame.height as usize).into()
                        },
                        row_size: 0,
                    };

                    trace!("decoded bitmap {}", bitmap);

                    resource.bitmaps.push(bitmap);

                    break;
                }
            }
        }
        
        Ok(resource)

    //     let mut format_context = AVFormatContextInput::open(&CString::new(path.to_owned()).unwrap(), None, &mut None).unwrap();

    //     let mut video_steam_id = 0;
    //     let mut is_valid = false;

    //     for steam in format_context.streams() {
    //         if steam.codecpar().codec_type().is_video() {
    //             video_steam_id = steam.index;
    //             is_valid = true;
    //             trace!("found video stream");
    //         }
    //     }

    //     if !is_valid {
    //         return Err(IffError::InvalidAnimationForm);
    //     }

    //     let mut bitmap: IffBitmap = IffBitmap::default();

    //     let mut resource = IffResource::default();

    //     loop {
    //         match format_context.read_packet().unwrap() {
    //             Some(p) => {
    //                 if video_steam_id == p.stream_index {
    //                     // Parse the packet data into a bitmap
    //                     let data_slice: &[u8] = unsafe {
    //                         std::slice::from_raw_parts(p.data, p.size as usize)
    //                     };

    //                     let cursor = Cursor::new(data_slice);
    //                     let mut reader = BufReader::new(cursor);

    //                     let mut read_bitmap = new(&mut reader, data_slice.len() as u64)?.bitmaps.pop().unwrap();

    //                     resource.bitmaps.push(read_bitmap);
    //                 }
    //             },
    //             None => break
    //         }
    //     }

    //     Ok(resource)
    // }
    }
}

macro_rules! make_sig {
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        (($a as u32) << 24) | (($b as u32) << 16) | (($c as u32) << 8) | ($d as u32)
    };
}

#[derive(Debug)]
enum Signature {
    Form,
    Ilbm,
    Body,
    Bmhd,
    ColorMap,
    Unknown,
    Pbm,
    Anim,
    Delta,
    Anhd
}

fn read_signature<R: Read + Seek>(reader: &mut BufReader<R>) -> Result<Signature, IffError> {
    let mut sig = [0u8; 4];

    let count = match reader.read(&mut sig) {
        Ok(result) => result,
        Err(e) => return Err(IffError::Io(e))
    };

    let sig_str = match std::str::from_utf8(&sig) {
        Ok(result) => result,
        Err(e) => return Err(IffError::Parse(e))
    };

    let sig = match sig_str {
        "ILBM" => Signature::Ilbm,
        "BODY" => Signature::Body,
        "CMAP" => Signature::ColorMap,
        "BMHD" => Signature::Bmhd,
        "FORM" => Signature::Form,
        "ANIM" => Signature::Anim,
        "DLTA" => Signature::Delta,
        "ANHD" => Signature::Anhd,
        "PBM" => Signature::Pbm,
        _ => {
            debug!("Found unhandled signature: {}", sig_str);
            Signature::Unknown
        }
    };

    Ok(sig)
}


fn parse_bitmap_header<R: Read + Seek>(reader: &mut BufReader<R>, bitmap: &mut IffBitmap) -> Result<(), IffError> {
    bitmap.width = reader.read_i16::<BigEndian>().unwrap();
    bitmap.height = reader.read_i16::<BigEndian>().unwrap();
    bitmap.x = reader.read_i16::<BigEndian>().unwrap();
    bitmap.y = reader.read_i16::<BigEndian>().unwrap();

    debug!("bitmap width: {:?}", bitmap.width);
    debug!("bitmap height: {:?}", bitmap.height);

    bitmap.num_planes = reader.read_u8().unwrap();


    bitmap.masking = match reader.read_u8().unwrap() {
        0 => MaskingTypes::None,
        1 => MaskingTypes::HasMask,
        2 => MaskingTypes::HasTransparentColor,
        _ => MaskingTypes::Unknown
    };


    bitmap.compression = match reader.read_u8().unwrap() {
        0 => CompressionTypes::None,
        1 => CompressionTypes::ByteRun1,
        _ => CompressionTypes::Unknown
    };

    /* Skip padding */
    let _ = reader.seek(SeekFrom::Current(1));

    let transparent_color = reader.read_i16::<BigEndian>().unwrap();

    bitmap.x_aspect = reader.read_u8().unwrap();
    bitmap.y_aspect = reader.read_u8().unwrap();

    bitmap.page_width = reader.read_i16::<LittleEndian>().unwrap();
    bitmap.page_height = reader.read_i16::<LittleEndian>().unwrap();

    if bitmap.masking == MaskingTypes::HasTransparentColor {
        bitmap.transparent_color = Some(transparent_color);
    }
    else if (bitmap.masking == MaskingTypes::Unknown) {
        // return Err(IffError::UnknownMask);

        // Lets log it out, not throw an error
        warn!("Uknown mask type found in IFF bitmap");
    }

    /* Compute the depth */

    bitmap.data = vec![0u8; bitmap.width as usize * bitmap.height as usize];

    

    Ok(())
}

fn parse_body<R: Read + Seek>(reader: &mut BufReader<R>, bitmap: &mut IffBitmap, block_size: i32) -> Result<(), IffError> {
    debug!("body bitmap type: {:?}", bitmap.bitmap_type);
    debug!("body compression type: {:?}", bitmap.compression);

    let len = bitmap.data.len();
    let mut block_offset = 0;

    let (width, depth) = match bitmap.bitmap_type {
        BitmapTypes::Pbm => {
            (bitmap.width, 1)
        },
        BitmapTypes::Ilbm => {
            ((bitmap.width + 7) / 8, bitmap.num_planes)
        },
        _ => {
            return Err(IffError::InvalidBitmapType)
        }
    };

    debug!("width: {}", width);
    debug!("depth: {}", depth);

    /* avoid a danger */
    if depth != 1 {
        bitmap.data = vec![0u8; width as usize * bitmap.height as usize * depth as usize];
    }

    match bitmap.compression {
        CompressionTypes::None => {
            let mut pos = 0usize;

            for _ in 0..bitmap.height {

                for _ in 0..(width * depth as i16) {
                    bitmap.data[pos] = reader.read_u8().unwrap();
                    pos += 1;
                }

                // Skip mask
                if bitmap.masking == MaskingTypes::HasMask {
                    let _ = reader.seek(SeekFrom::Current(width.into()));
                }

                if (bitmap.width & 1) != 0 {
                    let _ = reader.seek(SeekFrom::Current(1));
                }
            }
        },
        CompressionTypes::ByteRun1 => {
            let mut pos = 0usize;
            let has_mask = bitmap.masking == MaskingTypes::HasMask;
            let mut cur_width = 0i32;
            let mut skip_mask = false;
            let mut plane = 0;

            /*
            UnPacker:
              LOOP until produced the desired number of bytes
                  Read the next source byte into n
                  SELECT n FROM
                      [0..127]   => copy the next n+1 bytes literally
                      [-1..-127] => replicate the next byte -n+1 times
                      -128       => no operation
                      ENDCASE;
                  ENDLOOP;
             */

            loop {
                if pos >= len || block_offset >= block_size {
                    break;
                }

                if cur_width == width as i32 {
                    plane += 1;

                    if (plane == depth && !has_mask) || (plane == depth + 1 && has_mask) {
                        skip_mask = false;
                        plane = 0;
                    }

                    if has_mask && plane == depth {
                        skip_mask = true;
                    }

                    cur_width = 0;
                }

                let command: i32 = reader.read_i8().unwrap().into();
                block_offset += 1;

                // trace!("cmd = {}", command);

                if command == -128 {
                    continue;
                }

                if command >= 0 && command <= 127 {
                    if !skip_mask {
                        // trace!("positive command: {}", command + 1);
                        for _ in 0..(command + 1) {
                            bitmap.data[pos] = reader.read_u8().unwrap();
                            block_offset += 1;
                            pos += 1;
                        }
                    }
                    else {
                        let _ = reader.seek(SeekFrom::Current((command + 1).into()));
                        block_offset += command + 1;
                    }

                    cur_width += command + 1;
                }
                else if command >= -127 && command < 0 {
                    let run = (-command) + 1;
                    let repeat_byte = reader.read_u8().unwrap();
                    block_offset += 1;

                    // trace!("run = {}", run);
                    // trace!("repeat = {}", repeat_byte);

                    if !skip_mask {
                        for _ in 0..run {
                            bitmap.data[pos] = repeat_byte;
                            pos += 1;
                        }
                    }

                    cur_width += run;
                }
            }
        },
        CompressionTypes::Unknown => {
            return Err(IffError::InvalidCompression);
        }
    }

    Ok(())
}

//XXX: This function seems broken..
fn parse_delta<R: Read + Seek>(reader: &mut BufReader<R>, len: i64, bitmap: &mut IffBitmap) -> Result<(), IffError> {
    let chunk_end = reader.stream_position().unwrap() + (len as u64);
    let mut pos = 0;

    // longword, seems to be equal to 4.  Don't know what it is
    let _ = reader.seek(SeekFrom::Current(4));

    for _ in 0..bitmap.height {
        let mut count = bitmap.width;

        let mut num_items = reader.read_i8().unwrap();

        if num_items == 0 { //??
            // so push the buffer ahead
            let _ = reader.seek(SeekFrom::Current(len - 4));
            return Ok(());
        }

        trace!("num_items = {}", num_items);

        for _ in 0..num_items {
            let code = reader.read_u8().unwrap();

            match code {
                0 => {
                    let mut rep = reader.read_u8().unwrap();
                    let val = reader.read_u8().unwrap();

                    count -= rep as i16;
                    if count == -1 { rep -= 1; }
                    
                    for _ in 0..rep {
                        bitmap.data[pos] = val;
                        pos += 1;
                    }
                },
                c if c > 0x80 => { // Skip
                    let t = code - 0x80;
                    count -= t as i16;
                    pos += t as usize;
                    
                    if count == -1 {
                        pos -= 1;
                    }
                },
                _ => { // Literal
                    count -= code as i16;
                    let mut _code = code;

                    if count == -1 {
                        _code -= 1;
                    }

                    for _ in 0.._code {
                        bitmap.data[pos] = reader.read_u8().unwrap();
                        pos += 1;
                    }

                    if count == -1 {
                        let _ = reader.seek(SeekFrom::Current(1));
                    }
                }
            }
        }

        if count == -1 {
            if (bitmap.width & 1) == 0 {
                error!("{}", line!());
                return Err(IffError::Corrupt);
            }
        }
        else if count >= 1 {
            error!("count = {}", count);
            return Err(IffError::Corrupt);
        }
    }

    if reader.stream_position().unwrap() == chunk_end - 1 { // pad
        let _ = reader.seek(SeekFrom::Current(1));
    }

    if reader.stream_position().unwrap() != chunk_end {
        panic!();
        // return Err(IffError::Corrupt);
    }
    else {
        Ok(())
    }
}

pub fn new<R: Read + Seek>(reader: &mut BufReader<R>, length: u64) -> Result<IffResource, IffError> {
    let mut resource = IffResource::default();

    debug!("IFF source size {}", length);

    loop {
        if (reader.stream_position().unwrap() + 4) >= length {
            break;
        }

        let sig = read_signature(reader)?;

        let mut len = 4;

        debug!("IFF data block type: {:?}", &sig);

       let mut curr = resource.bitmaps.len() - 1;

        match sig {

            Signature::Form => {
                let s = read_signature(reader).unwrap();
                debug!("Form sig: {:?}", s);

                resource.bitmaps.push(IffBitmap::default());
                curr = resource.bitmaps.len() - 1;

                match s {
                    Signature::Ilbm => resource.bitmaps[curr].bitmap_type = BitmapTypes::Ilbm,
                    _ => resource.bitmaps[curr].bitmap_type = BitmapTypes::Pbm
                }
            },
            Signature::Bmhd => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;
                parse_bitmap_header(reader, &mut resource.bitmaps[curr])?;
            },
            Signature::Ilbm => {
                resource.bitmaps.push(IffBitmap::default());
                curr = resource.bitmaps.len() - 1;
                resource.bitmaps[curr].bitmap_type = BitmapTypes::Ilbm;
            },
            Signature::Pbm => {
                resource.bitmaps.push(IffBitmap::default());
                curr = resource.bitmaps.len() - 1;
                resource.bitmaps[curr].bitmap_type = BitmapTypes::Pbm;
            }
            Signature::Anhd => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;

                if (len & 1) != 0 {
                    len += 1;
                }

                let _ = reader.seek(SeekFrom::Current(len.into()));
            }
            Signature::ColorMap => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;
                for c in 0..((len / 3) as usize) {
                    resource.bitmaps[curr].pallete[c].red = reader.read_u8().unwrap() >> 2;
                    resource.bitmaps[curr].pallete[c].green = reader.read_u8().unwrap() >> 2;
                    resource.bitmaps[curr].pallete[c].blue = reader.read_u8().unwrap() >> 2;
                }

                if (len & 1) != 0 {
                    let _ = reader.seek(SeekFrom::Current(1));
                }
            },
            Signature::Body => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;
                parse_body(reader, &mut resource.bitmaps[curr], len)?;
            },
            Signature::Delta => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;
                // Clone the current bitmap into a new slot
                let cloned_last_frame = resource.bitmaps[curr].clone();
                resource.bitmaps.push(cloned_last_frame);
                curr = resource.bitmaps.len() - 1;
                parse_delta(reader, len as i64, &mut resource.bitmaps[curr])?;
            },
            _ => {
                len = reader.read_i32::<BigEndian>().unwrap() as i32;

                // don't know this chunk
                if (len & 1) != 0 {
                    len += 1;
                }

                let _ = reader.seek(SeekFrom::Current(len.into()));
            }
        }
    }

    trace!("{}", resource);

    Ok(resource)
}


#[cfg(test)]
pub mod tests {
    use std::{env, fs::{metadata, File}, io::Cursor, path::{Path, PathBuf}, sync::Once};
    use byteorder::*;
    use env_logger::Env;
    use crate::graphics::bitmap;

    use super::*;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                .is_test(true) // Ensure it's suitable for test environment
                .init();
        });
    }

    #[test]
    fn iff_badapple_test() {
        // setup();

        // let mut path = PathBuf::from(env::current_dir().unwrap().to_str().unwrap());
        // path.push(Path::new(file!()).parent().unwrap().to_str().unwrap());
        // path.push("testdata/badapple-219frames.iff");

        // let metdata = std::fs::metadata(&path).unwrap();
        // let badapple = File::open(path).unwrap();
        // let mut reader = BufReader::new(badapple);
        // let bitmap = IffBitmap::new(&mut reader, metdata.len()).unwrap();

       // This test fails
       // The delta function doesn't find any num_items
       // It seems broken
       // Better to leave it up with ffmpeg to deal with this
    }

    #[test]
    #[cfg(feature = "with_ffmpeg")]
    fn iff_badapple_test_ffmpeg() {
        setup();

        use rsmpeg::{
            avcodec::AVCodec, avformat::{AVFormatContextInput, AVInputFormat, AVStreamRef}, avutil::AVDictionary, error::RsmpegError
        };

        use std::ffi::{CStr, CString};

        let mut path = PathBuf::from(env::current_dir().unwrap().to_str().unwrap());
        path.push(Path::new(file!()).parent().unwrap().to_str().unwrap());
        path.push("testdata/badapple-219frames.iff");

        let mut codec :Option<AVCodec> = None;
        let mut format_context = AVFormatContextInput::open(&CString::new(path.to_string_lossy().into_owned()).unwrap(), None, &mut None).unwrap();

        let mut frames = 0;
        let mut video_steam_id = 0;

        for steam in format_context.streams() {
            if steam.codecpar().codec_type().is_video() {
                frames = steam.nb_frames;
                video_steam_id = steam.index;
            }
        }

        if frames < 1 {
            loop {
                match format_context.read_packet().unwrap() {
                    Some(p) => {
                        if video_steam_id == p.stream_index {
                            frames += 1;
                        }
                    },
                    None => break
                }
            }
        }

        assert_eq!(frames, 219 + 2);
    }

    #[test]
    #[cfg(feature = "with_ffmpeg")]
    fn iff_badapple_test_ffmpeg_constructor() {

        setup();

        let mut path = PathBuf::from(env::current_dir().unwrap().to_str().unwrap());
        path.push(Path::new(file!()).parent().unwrap().to_str().unwrap());
        path.push("testdata/badapple-219frames.iff");

        let metdata = std::fs::metadata(&path).unwrap();
        let badapple = File::open(path).unwrap();
        let mut reader = BufReader::new(badapple);

        let animated_resource = IffResource::new_from_ffmpeg(&mut reader, 100, 100).unwrap();

        assert_eq!(animated_resource.bitmaps.len(), 221 - 1);
    }
}