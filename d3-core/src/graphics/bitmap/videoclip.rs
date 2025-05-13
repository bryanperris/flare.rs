use crate::{common::SharedMutRef, game::terrain::TERRAIN_WIDTH, graphics::{bitmap::{scale_bitmap_16}, texture::TextureSizeType, TEXTURE_HEIGHT, TEXTURE_WIDTH}, string::D3String};
use core::str;
use std::{io::{BufReader, Read, Seek}, os::unix::raw::off_t};
use byteorder::{LittleEndian, ReadBytesExt, BigEndian};

use super::bitmap::{Bitmap16, BitmapFormat, ScaleableBitmap16};

use bitflags::bitflags;
use anyhow::Result;
use log;

const MAX_CLIPS: usize = 200;
const MAX_FRAMES: usize = 50;
const DEFAULT_FRAMETIME: f32 = 0.07;

pub enum VideoClipFormat {
    IFL,
    ABM
}

// TODO: Lazy implementations for frames

#[derive(Debug)]
pub struct VideoClip {
    name: D3String,
    frames: Vec<Box<dyn Bitmap16>>,
    frame_time: f32, // time (in seconds) of each frame
}

pub type BitmapLoader<B: Bitmap16 + ScaleableBitmap16 + Clone + 'static> = dyn Fn(&str) -> Option<B>;

impl VideoClip {
    pub fn new<R: Read + Seek, B: Bitmap16 + ScaleableBitmap16 + Clone + 'static>(name: D3String, format: VideoClipFormat, reader: &mut BufReader<R>, len: usize, texture_size: TextureSizeType, is_mipped: bool, bitmap_loader: &BitmapLoader<B>) -> Result<Self> {
        let name = name.to_string().unwrap();

        let vclip = match format {
            VideoClipFormat::ABM => panic!("format unsupported"),
            VideoClipFormat::IFL => load_ifvl_clip(&name, reader, len, texture_size, is_mipped, bitmap_loader)
        };

        vclip
    }

    pub fn name(&self) -> &D3String {
        &self.name
    }

    pub fn frametime(&self) -> f32 {
        self.frame_time
    }

    pub fn frames(&self) -> &[Box<dyn Bitmap16>] {
        self.frames.as_slice()
    }

    pub fn get_frame_bitmap(&self, frame: usize) -> &Box<dyn Bitmap16> {
        &self.frames[frame]
    }

    // XXX: I don't think we even care, once a vclip is dropped
    //      So will the the bitmap refs
    // pub fn free_residency(&mut self) {
    //     self.frames.clear();
    // }

    // pub fn not_resident(&self) -> bool {
    //     self.frames.len() == 0
    // }

    // TOOD: We should do lazy loading with the hog
    // Instead of the old D3 paging system
}

/// Allocs and loads a vclip from a 3DS ILS file
fn load_ifvl_clip<R, B>(name: &str, reader: &mut BufReader<R>, len: usize, texture_size: TextureSizeType, is_mipped: bool, bitmap_loader: &BitmapLoader<B>) -> Result<VideoClip>
    where R: Read + Seek,
          B: Bitmap16 + ScaleableBitmap16 + Clone + 'static  {

    let start = reader.stream_position().unwrap();

    let mut curline_read = [0u8; 200];

    let mut frames: Vec<Box<dyn Bitmap16>> = Vec::new();
    let mut name = "".to_string();

    loop {
        if (reader.stream_position().unwrap() - start) >= len as u64 {
            break;
        }

        // Read a line and parse it
        reader.read(&mut curline_read).unwrap();
        let curline = D3String::from_slice(&curline_read);

        match curline.char_at(0) {
            ';' => continue,
            ' ' => continue,
            _ => {}
        }

        match curline.char_at(1) {
            ';' => continue,
            ' ' => continue,
            _ => {}
        }

        if !curline.char_at(0).is_alphanumeric() {
            continue;
        }
        else if curline.char_at(0) == '$' {
            let mut new_command = [0; 50];

            for i in 0..new_command.len() {
                if curline.char_at(i + 1) == '=' {
                    break;
                }

                new_command[i] = curline.byte_at(i + 1);

                if i == new_command.len() - 1 {
                    return Err(anyhow!("bad command in IFL!"));
                }
            }

            // Advance to the data
            let new_command = std::str::from_utf8(&new_command).unwrap_or("");

            if "TIME".eq_ignore_ascii_case(&new_command) {
                let play_time = &curline[new_command.len()+1..];
                let play_time = str::from_utf8(&play_time).unwrap_or("");
                let play_time: f32 = play_time.parse().expect("Failed to parse play time");

                // Assert that the play time is non-negative
                assert!(play_time >= 0.0, "Play time must be non-negative");
            }
        }
        else {
            let mut bitmap_name = "".to_string();
            let mut lastslash = None;

            let line = curline.to_string().unwrap();

            for i in 0..curline.len() {
                if curline.char_at(i) == '\\' {
                    lastslash = Some(i);
                }
            }

            if lastslash.is_none() {
                 bitmap_name = name.to_string();
            }
            else {
                bitmap_name = line;
            }

            let mut bitmap = Box::new(bitmap_loader(&bitmap_name).unwrap());

            name = bitmap_name.to_string();
            let name = format!("{}.oaf", name);
            trace!("bitmap name is {}", &bitmap_name);

            let w;
            let h;

            match texture_size {
                TextureSizeType::Normal => {
                    w = TEXTURE_WIDTH;
                    h = TEXTURE_HEIGHT;
                },
                TextureSizeType::Small => {
                    w = TEXTURE_WIDTH / 2;
                    h = TEXTURE_HEIGHT / 2;
                },
                TextureSizeType::Tiny => {
                    w = TERRAIN_WIDTH / 4;
                    h = TEXTURE_HEIGHT / 4;
                },
                _ => {
                    w = bitmap.width();
                    h = bitmap.height();
                }
            }

            let additional_mem = if is_mipped {
                (w * h) / 3
            } else {
                0
            };

            if w != bitmap.width() || h != bitmap.height() {
                let scaled_bitmap_result = scale_bitmap_16(bitmap.as_ref(), is_mipped, w, h, additional_mem);
                bitmap = Box::new(scaled_bitmap_result.unwrap());
            }

            frames.push(bitmap);
        }
    }

    Ok(VideoClip {
        name: D3String::from(name),
        frames: frames,
        frame_time: DEFAULT_FRAMETIME
    })
}