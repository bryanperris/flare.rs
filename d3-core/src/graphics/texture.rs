use std::{collections::HashSet, sync::Arc};

use bitflags::bitflags;

use crate::{common::{new_shared_mut_ref, SharedMutRef}, graphics::bitmap::MemBitmap16, string::D3String};

use super::{bitmap::{videoclip::VideoClip, Bitmap16}, bumpmap::BumpMap16, procedural::ProceduralBitmap16, GpuMemoryResource, TEXTURE_HEIGHT, TEXTURE_WIDTH};

use anyhow::Result;

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct TextureFlags: u32 {
        const NONE = 0;
        const VOLATILE = 1;
        const WATER = 1 << 1;
        const METAL = 1 << 2;       // Shines like metal
        const MARBLE = 1 << 3;      // Shines like marble
        const PLASTIC = 1 << 4;     // Shines like plastic
        const FORCEFIELD = 1 << 5;
        const ANIMATED = 1 << 6;
        const DESTROYABLE = 1 << 7;
        const EFFECT = 1 << 8;
        const HUD_COCKPIT = 1 << 9;
        const MINE = 1 << 10;
        const TERRAIN = 1 << 11;
        const OBJECT = 1 << 12;
        const TEXTURE_64 = 1 << 13;
        const TMAP2 = 1 << 14;
        const TEXTURE_32 = 1 << 15;
        const FLY_THRU = 1 << 16;
        const PASS_THRU = 1 << 17;
        const PING_PONG = 1 << 18;
        const LIGHT = 1 << 19;
        const BREAKABLE = 1 << 20;  // Breakable (as in glass)
        const SATURATE = 1 << 21;
        const ALPHA = 1 << 22;
        const DONTUSE = 1 << 23;
        const PROCEDURAL = 1 << 24;
        const WATER_PROCEDURAL = 1 << 25;
        const FORCE_LIGHTMAP = 1 << 26;
        const SATURATE_LIGHTMAP = 1 << 27;
        const TEXTURE_256 = 1 << 28;
        const LAVA = 1 << 29;
        const RUBBLE = 1 << 30;
        const SMOOTH_SPECULAR = 1 << 31;

        const TEXTURE_TYPES = Self::MINE.bits() | Self::TERRAIN.bits() | Self::OBJECT.bits() |
                              Self::EFFECT.bits() | Self::HUD_COCKPIT.bits() | Self::LIGHT.bits();

        const SPECULAR = Self::METAL.bits() | Self::MARBLE.bits() | Self::PLASTIC.bits();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TextureSizeType {
    None,
    Normal,
    /// 1/4 of a nomal texture
    Small,
    /// 1/8 of a normal texture
    Tiny,
    /// double the size of a normal texture
    Huge,
}


#[derive(Debug, Clone)]
pub enum BitmapSource {
    Bitmap16(SharedMutRef<dyn Bitmap16>),
    VideoClip(SharedMutRef<VideoClipSource>),
    Procedural(ProceduralSource)
}

#[derive(Debug)]
pub struct VideoClipSource {
    bitmap: VideoClip,
    frame_offset: usize,
}

impl VideoClipSource {
    fn step_frame(&mut self, speed: f32, gametime: f32, framenum: usize, fmod: usize) {
        let count = self.bitmap.frames().len();
        let frametime = speed / count as f32;
        let current_frametime = gametime / frametime;
        self.frame_offset = current_frametime as u32 as usize;
        self.frame_offset += framenum;
        self.frame_offset %= fmod;
    }

    fn step_frame_ping_pong(&mut self, speed: f32, gametime: f32, framenum: usize, fmod: usize) {
        self.step_frame(speed, gametime, framenum, fmod * 2);

        let count = self.bitmap.frames().len();

        if self.frame_offset >= count {
            self.frame_offset = (count - 1) - (self.frame_offset % count);
        } else {
            self.frame_offset %= count;
        }
    }
}

impl Bitmap16 for VideoClipSource {
    fn data(&self) -> &[u16] {
        self.bitmap.get_frame_bitmap(self.frame_offset).data()
    }

    fn width(&self) -> usize {
        self.bitmap.get_frame_bitmap(self.frame_offset).width()
    }

    fn height(&self) -> usize {
        self.bitmap.get_frame_bitmap(self.frame_offset).height()
    }

    fn mip_levels(&self) -> usize {
        self.bitmap.get_frame_bitmap(self.frame_offset).mip_levels()
    }

    fn flags(&self) -> &super::bitmap::BitmapFlags {
        self.bitmap.get_frame_bitmap(self.frame_offset).flags()
    }

    fn name(&self) -> &D3String {
        self.bitmap.name()
    }

    fn format(&self) -> super::bitmap::BitmapFormat {
        self.bitmap.get_frame_bitmap(self.frame_offset).format()
    }

    fn make_funny(&mut self) {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct ProceduralSource {
    bitmap: SharedMutRef<ProceduralBitmap16>,
    last_frame: usize,
    last_evalution_time: u128,
    evaluation_time: u128
}

impl ProceduralSource {
    pub fn new(bitmap: ProceduralBitmap16) -> Self {
        Self {
            bitmap: crate::common::new_shared_mut_ref(bitmap),
            evaluation_time: 0,
            last_evalution_time: 0,
            last_frame: 0
        }
    }
}

#[derive(Debug, Clone)]
pub struct Texture16 {
    pub name: D3String,
    pub flags: TextureFlags,
    pub bitmap_source: Option<BitmapSource>,
    pub destroy_bitmap_source: Option<BitmapSource>,
    pub bump_map: Option<BumpMap16>,
    pub updated: bool,
    pub size: TextureSizeType,

    pub damage: i32,
    pub reflectivity: f32,
    pub corona_type: u8,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub alpha: f32, // 0..1

    // How many times this texture slides during a second
    pub slide_u: f32,
    pub slide_v: f32,

    /// how fast this texture animates
    pub speed: f32,

    pub sound: (),
    pub sound_volume: f32,
}

impl Default for Texture16 {
    fn default() -> Self {
        Self {
            flags: TextureFlags::NONE,
            alpha: 1.0,
            speed: 1.0,
            reflectivity: 0.6,
            ..Default::default()
        }
    }
}

impl GpuMemoryResource for Texture16 {
    fn mark_updated(&mut self) {
        self.updated = false;
    }

    fn is_updated(&self) -> bool {
        self.updated
    }
}

impl Texture16 {
    pub fn compute_procedural_size(&self) -> (usize, usize) {
        if self.flags.contains(TextureFlags::TEXTURE_64) {
            ( 64, 64 )
        }
        else if self.flags.contains(TextureFlags::TEXTURE_32) {
            ( 32, 32 )
        }
        else {
            ( 128, 128 )
        }
    }

    pub fn contains_named_bitmap(&mut self, name: &D3String) -> bool {
        if self.bitmap_source.is_some() {
            let bitmap_source = self.bitmap_source.as_ref().unwrap();

            let is_match = match bitmap_source {
                BitmapSource::Bitmap16(x) => x.borrow().name().eq(name),
                BitmapSource::VideoClip(x) => {
                    let clip_src = x.borrow();
                    let clip = &clip_src.bitmap;

                    let mut is_match = false;

                    for frame in clip.frames() {
                        if frame.name().eq(name) {
                            is_match = true;
                            break;
                        }
                    }

                    is_match
                },
                BitmapSource::Procedural(p) => {
                    let bitmap = &p.bitmap;

                    if bitmap.borrow().base_bitmap().is_some() {
                        let b = bitmap.borrow();
                        let bitmap_ref = b.base_bitmap();
                        let bitmap = bitmap_ref.as_ref().unwrap();
                        return bitmap.name().eq(name);
                    }
                    
                    false
                }
            };
        }

        if self.destroy_bitmap_source.is_some() {
            let bitmap_source = self.destroy_bitmap_source.as_ref().unwrap();

            let is_match = match bitmap_source {
                BitmapSource::Bitmap16(x) => x.borrow().name().eq(name),
                BitmapSource::VideoClip(x) => {
                    let clip_src = x.borrow();
                    let clip = &clip_src.bitmap;

                    let mut is_match = false;

                    for frame in clip.frames() {
                        if frame.name().eq(name) {
                            is_match = true;
                            break;
                        }
                    }

                    is_match
                },
                BitmapSource::Procedural(p) => {
                    let bitmap = &p.bitmap;

                    if bitmap.borrow().base_bitmap().is_some() {
                        let b = bitmap.borrow();
                        let bitmap_ref = b.base_bitmap();
                        let bitmap = bitmap_ref.as_ref().unwrap();
                        return bitmap.name().eq(name);
                    }
                    
                    false
                }
            };
        }

       false
    }

    pub fn step_animation(&mut self, gametime: f32, frame_number: usize, force: bool) {
        let mut mark_updated = false;

        if self.bitmap_source.is_some() {
            { 
                let bitmap = self.bitmap_source.as_mut().unwrap();

                match bitmap {
                    BitmapSource::Bitmap16(ref_cell) => {},
                    BitmapSource::VideoClip(ref_cell) => {
                        let mut vclip = ref_cell.borrow_mut();
                        let frame_count = vclip.bitmap.frames().len();
                        
                        if (self.flags & TextureFlags::PING_PONG) == TextureFlags::PING_PONG {
                            vclip.step_frame_ping_pong(self.speed, gametime, frame_number, frame_count);
                        }
                    },
                    BitmapSource::Procedural(p) => {
                        let mut do_step = true;
                        
                        if p.last_frame == p.bitmap.borrow().frame_count() {
                            do_step = false;
                        }
                        
                        if p.bitmap.borrow().get_ticks() < (p.last_evalution_time + p.evaluation_time) {
                            do_step = false;
                        }
                        
                        if !force && !p.bitmap.borrow().is_procedurals_enabled() {
                            if p.bitmap.borrow().get_ticks() < (p.last_evalution_time + 10) {
                                do_step = false;
                            }
                        }

                        if do_step {
                            mark_updated = true;
                            p.last_frame = p.bitmap.borrow().frame_count();
                            p.last_evalution_time = p.bitmap.borrow().get_ticks();
                        }
                    }
                }
            }
        }

        if self.destroy_bitmap_source.is_some() {
            {
                let bitmap = self.destroy_bitmap_source.as_mut().unwrap();

                match bitmap {
                    BitmapSource::Bitmap16(ref_cell) => {},
                    BitmapSource::VideoClip(ref_cell) => {},
                    BitmapSource::Procedural(p) => {
                        let mut do_step = true;
                        
                        if p.last_frame == p.bitmap.borrow().frame_count() {
                            do_step = false;
                        }
                        
                        if p.bitmap.borrow().get_ticks() < (p.last_evalution_time + p.evaluation_time) {
                            do_step = false;
                        }
                        
                        if !force && !p.bitmap.borrow().is_procedurals_enabled() {
                            if p.bitmap.borrow().get_ticks() < (p.last_evalution_time + 10) {
                                do_step = false;
                            }
                        }

                        if do_step {
                            mark_updated = true;
                            p.last_frame = p.bitmap.borrow().frame_count();
                            p.last_evalution_time = p.bitmap.borrow().get_ticks();
                        }
                    }
                }
            }
        }

        if mark_updated {
            self.mark_updated();
        }
    }

    pub fn source_bitmap(&self) -> Option<SharedMutRef<dyn Bitmap16>> {
        if self.bitmap_source.is_some() {
            let bitmap = self.bitmap_source.as_ref().unwrap();

            match bitmap {
                BitmapSource::Bitmap16(ref_cell) => {
                    return Some(ref_cell.clone());
                },
                BitmapSource::VideoClip(ref_cell) => {
                    return Some(ref_cell.clone());
                },
                BitmapSource::Procedural(procedural_source) => {
                    return Some(procedural_source.bitmap.clone());
                }
            }
        }

        None
    }

    pub fn destroy_bitmap(&self) -> Option<SharedMutRef<dyn Bitmap16>> {
        if self.destroy_bitmap().is_some() {
            let bitmap = self.destroy_bitmap_source.as_ref().unwrap();

            match bitmap {
                BitmapSource::Bitmap16(ref_cell) => {
                    return Some(ref_cell.clone());
                },
                BitmapSource::VideoClip(ref_cell) => {
                    return Some(ref_cell.clone());
                },
                BitmapSource::Procedural(procedural_source) => {
                    return Some(procedural_source.bitmap.clone());
                }
            }
        }
        
        None
    }

    pub fn should_scale_bitmap<T: Bitmap16>(&self, bitmap: &T) -> Option<(usize, usize)> {
        let (w, h) = match self.size {
            TextureSizeType::None => todo!(),
            TextureSizeType::Normal => {
                ( TEXTURE_WIDTH, TEXTURE_HEIGHT )
            },
            TextureSizeType::Small => {
                ( TEXTURE_WIDTH / 2, TEXTURE_HEIGHT / 2 )
            },
            TextureSizeType::Tiny => {
                ( TEXTURE_WIDTH / 4, TEXTURE_HEIGHT / 4 )
            },
            TextureSizeType::Huge => {
                ( TEXTURE_WIDTH * 2, TEXTURE_HEIGHT * 2 )
            },
        };

        if w != bitmap.width() || h != bitmap.height() {
            return Some((w, h));
        }

        None
    }

    pub fn build_bumpmaps(&mut self) {
        if let bitmap_source = self.bitmap_source.as_ref().unwrap() {
            match bitmap_source {
                BitmapSource::Bitmap16(ref_cell) => {
                    let bitmap = ref_cell.borrow();

                    let mut bump_map = BumpMap16::new(bitmap.width(), bitmap.height());
                    let mut buffer = vec![0i8; bitmap.width() * bitmap.height()];

                    // Create the grayscale
                    for i in 0..bitmap.height() {
                        for t in 0..bitmap.width() {
                            let color = bitmap.data()[i * bump_map.width() + t];

                            let red = ((color >> 10) & 0x1F) << 3;
                            let green = ((color >> 5) & 0x1F) << 3;
                            let blue = (color & 0x1F) << 3;

                            let gray = 0.39 * red as f32 + 0.60 * green as f32 + 0.11 * blue as f32;

                            buffer[i * bitmap.width() + t] = gray as i8;
                        }
                    }

                    let bump_map_data = bump_map.data_mut();

                    let mut src = 0;
                    let mut dst = 0;
                    for i in 0..bitmap.height() {
                        dst = i + bitmap.width();

                        for t in 0..bitmap.width() {
                            // Get current pixe, *3 for 24 bits src
                            let v00 = buffer[i * bitmap.width() + t];

                            // Special case for last column
                            let v01 = if t == bitmap.width()- 1 {
                                // Get pixel to the right
                                buffer[i * bitmap.width() + t]
                            } else {
                                // Get pixel to the right
                                buffer[i + bitmap.height() + t + 1]
                            };

                            // Special case for last row
                            let v10 = if t == bitmap.height() - 1 {
                                // Get pixel one line below
                                buffer[i * bitmap.width() + t]
                            } else {
                                // Get pixel one line below
                                buffer[((i + 1) * bitmap.width()) + t]
                            };

                            // The delta U value
                            let u = v00 as i32 - v01 as i32;
                            
                            // The delta V value
                            let v = v00 as i32 - v10 as i32;

                            bump_map_data[dst] = u as i8 as u16;
                            bump_map_data[dst + 1] = u as i8 as u16;

                            dst += 2;
                        }
                    }

                    self.bump_map = Some(bump_map);
                },
                _ => {}
            }
        }
    }
}
