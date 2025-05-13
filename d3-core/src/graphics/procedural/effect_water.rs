use crate::{common::SharedMutRef, graphics::{bitmap::Bitmap16, OPAQUE_FLAG}};
use core::marker::PhantomData;
use std::{fmt::Debug};

use super::{place_point, ps_rand, BaseEmitter, DoubleBufferStorage, EmittedElement, EmitterEffect, ProceduralBitmap16, BRIGHT_COLOR, PROC_SIZE};

const NUM_WATER_SHADES: usize = 256;

#[derive(Debug, Copy, Clone, PartialEq)]
enum WaterDrawType {
    NoLight,
    Light(i32)
}

trait WaterEffectVariantClone {
    fn clone_box(&self) -> Box<dyn WaterEffectVariant>;
}

pub trait WaterEffectVariant: Debug + WaterEffectVariantClone {
    fn step(&self, context: &mut super::Context, memory: &mut DoubleBufferStorage);
}

impl<T> WaterEffectVariantClone for T
where
    T: 'static + WaterEffectVariant + Clone,
{
    fn clone_box(&self) -> Box<dyn WaterEffectVariant> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn WaterEffectVariant> {
    fn clone(&self) -> Box<dyn WaterEffectVariant> {
        self.clone_box()
    }
}

type WATER_HI = [[u16; 256]; NUM_WATER_SHADES];
type WATER_LO = [[u8; 256]; NUM_WATER_SHADES];

struct WaterTable {
    hi: WATER_HI,
    lo: WATER_LO
}

use once_cell::sync::Lazy;

static WATER_LUT: Lazy<WaterTable> = Lazy::new(|| {
    let mut table = WaterTable {
        hi: [[0u16; 256]; NUM_WATER_SHADES],
        lo: [[0u8; 256]; NUM_WATER_SHADES]
    };

    for i in 0..NUM_WATER_SHADES {
        let norm = i as f32 / (NUM_WATER_SHADES - 1) as f32;
        let lo_norm = f32::min(1.0, (norm / 0.5) * 1.0);
        let hi_norm = f32::max(0.0, ((norm - 0.5) / 0.5) * 1.0);

        for rcount in 0..32 {
            for gcount in 0..4 {
                let index = (rcount * 4) + gcount;
                let fr = rcount as f32;
                let r = f32::min(fr * lo_norm + (31.0 * hi_norm), 31.0);

                let r = (r.trunc() as u32) << 10;

                table.hi[i][index] = OPAQUE_FLAG | r as u16;
            }
        }

        for bcount in 0..32 {
            for gcount in 0..8 {
                let index = gcount * 32 + bcount;
                let b = f32::min(31.0, bcount as f32 * lo_norm + (31.0 * hi_norm));
                table.lo[i][index] = b as u8;
            }
        }

        for gcount in 0..8 {
            let g = f32::min(7.0, (gcount as f32 * lo_norm) + (7.0 * hi_norm)) as u32;

            for t in 0..32 {
                let index = gcount * 32 + t;
                table.lo[i][index] |= (g << 5) as u8;
            }
        }

        for gcount in 0..4 {
            let fg = gcount * 8;
            let g = f32::min((fg as f32 * lo_norm) + (24.0 * hi_norm), 24.0) as u32;

            for t in 0..32 {
                let index = t * 4 + gcount;
                table.hi[i][index] |= (g << 5) as u16;
            }
        }
    }

    table
});


#[derive(Debug, Clone)]
pub struct WaterEffect {
    draw_type: WaterDrawType,
    thickness: u8,
    easter_egg_ref: Option<SharedMutRef<dyn Bitmap16>>,
    effect: Box<dyn WaterEffectVariant>,
}

pub enum WaterVariant {
    V1,
    V2
}

impl WaterEffect {
    pub fn new<W: WaterEffectVariant + 'static>(effect_variant: W) -> Self
    where Self: Sized {
        Self {
            thickness: 0,
            draw_type: WaterDrawType::NoLight,
            easter_egg_ref: None,
            effect: Box::new(effect_variant)
        }
    }

    pub fn set_light(&mut self, light: i32) {
        if light > 0 {
            self.draw_type = WaterDrawType::Light(light)
        }
        else {
            self.draw_type = WaterDrawType::NoLight;
        }
    }

    pub fn enable_easter_egg(&mut self, easter_egg_bitmap_ref: &SharedMutRef<dyn Bitmap16>) {
        self.easter_egg_ref = Some(easter_egg_bitmap_ref.clone())
    }

    pub fn disable_easter_egg(&mut self) {
        self.easter_egg_ref = None;
    }

    pub fn set_thickness(&mut self, thickness: u8) {
        self.thickness = thickness;
    }

    fn calc_water(&mut self, variant: WaterVariant, density: i32, memory: &mut DoubleBufferStorage) {
        let (mut f, mut b) = memory.take_memory();

        let old;
        let new;

        unsafe {
            old = std::slice::from_raw_parts_mut(
                f.as_mut_ptr() as *mut i16, f.len()
            );

            new = std::slice::from_raw_parts_mut(
                b.as_mut_ptr() as *mut i16, b.len()
            );
        }

        // Do main block
        for y in 1..(PROC_SIZE - 1) {
            for x in 1..(PROC_SIZE - 1) {
                let offset = y * PROC_SIZE + x;
        
                let mut v = old[offset.wrapping_add(PROC_SIZE)];
                v = v.wrapping_add(old[offset.wrapping_sub(PROC_SIZE)]);
                v = v.wrapping_add(old[offset.wrapping_add(1)]);
                v = v.wrapping_add(old[offset.wrapping_sub(1)]);
        
                let new_h = match variant {
                    WaterVariant::V1 => {
                        v.wrapping_shr(1)
                    },
                    WaterVariant::V2 => {
                        v = v.wrapping_add(old[offset.wrapping_sub(PROC_SIZE).wrapping_sub(1)]);
                        v = v.wrapping_add(old[offset.wrapping_sub(PROC_SIZE).wrapping_add(1)]);
                        v = v.wrapping_add(old[offset.wrapping_add(PROC_SIZE).wrapping_sub(1)]);
                        v = v.wrapping_add(old[offset.wrapping_add(PROC_SIZE).wrapping_add(1)]);
                        v.wrapping_shr(2)
                    },
                    _ => panic!("invalid water variant"),
                }
                .wrapping_sub(new[offset]) as i32;
        
                new[offset] = (new_h.wrapping_sub(new_h.wrapping_shr(density as u32))) as i16;
            }
        }

        let size = PROC_SIZE as i32;

        for y in 0..PROC_SIZE {
            let up = if y == 0 { -((size - 1) * size) } else { size } as usize;
            let down = if y == PROC_SIZE - 1 { -((size  - 1) * size) } else { size } as usize;

            let mut x = 0;
            while x < PROC_SIZE {
                if (y != 0 && y != PROC_SIZE - 1) && x != 0 && x != PROC_SIZE - 1 {
                    if x == 1 { // Border skip, left to right
                        x = PROC_SIZE - 2;
                    }

                    x += 1;
                    continue;
                }

                let left = if x == 0 { -(size - 1) } else { 1 } as usize;
                let right = if x == PROC_SIZE - 1 { -(size - 1) } else { 1 } as usize;
                let offset = y * PROC_SIZE + x;

                let mut v = 0i32;
                v = v.wrapping_add(old[offset.wrapping_add(down)] as i32);
                v = v.wrapping_add(old[offset.wrapping_sub(up)] as i32);
                v = v.wrapping_add(old[offset.wrapping_add(right)] as i32);
                v = v.wrapping_add(old[offset.wrapping_sub(left)] as i32);

                let new_h = match variant {
                    WaterVariant::V1 => {
                        v.wrapping_shr(1)
                    }
                    WaterVariant::V2 => {
                        v = v.wrapping_add(old[offset.wrapping_sub(up).wrapping_sub(left)] as i32);
                        v = v.wrapping_add(old[offset.wrapping_sub(up).wrapping_add(right)] as i32);
                        v = v.wrapping_add(old[offset.wrapping_add(down).wrapping_sub(left)] as i32);
                        v = v.wrapping_add(old[offset.wrapping_add(down).wrapping_add(right)] as i32);
                        v.wrapping_shr(2)
                    },
                    _ => panic!("invalid water variant")
                }.wrapping_sub(new[offset] as i32);

                new[offset] = (new_h.wrapping_sub(new_h.wrapping_shr(density as u32))) as i16;

                x += 1;
            }
        }

        memory.replace_memory(f, b);
    }

    fn draw_water<'b>(&mut self, draw_type: WaterDrawType, bitmap_ref: &SharedMutRef<dyn Bitmap16>, dest_bitmap: &mut [u16], memory: &mut DoubleBufferStorage) {
        let bitmap = bitmap_ref.borrow_mut();
        let (f, b) = memory.take_memory();

        let ptr: &[i16];

        unsafe {
            ptr = std::slice::from_raw_parts(
                f.as_ptr() as *const i16, f.len()
            );
        }

        let mut offset = 0;

        match draw_type {
            WaterDrawType::NoLight => {
                for y in 0..PROC_SIZE {
                    for x in 0..PROC_SIZE {
                        let dx: i16 = std::cmp::max(0, if x == PROC_SIZE - 1 {
                            ptr[offset] - ptr[offset - (PROC_SIZE - 1)]
                        } else {
                            ptr[offset] - ptr[offset + 1]
                        });
        
                        let dy: i16 = std::cmp::max(0, if y == PROC_SIZE - 1 { 
                            ptr[offset] - ptr[offset - ((PROC_SIZE - 1) * PROC_SIZE)]
                        } else {
                            ptr[offset] - ptr[offset + PROC_SIZE]
                        });
        
                        let x_offset = (x + (dx >> 3) as usize) % PROC_SIZE;
                        let y_offset = (y + (dy >> 3) as usize) % PROC_SIZE;
        
                        let src_pixel = bitmap.data()[y_offset * PROC_SIZE + x_offset];
                        dest_bitmap[offset] = src_pixel;
        
                        offset += 1;
                    }
                }
            },
            WaterDrawType::Light(lightval) => {
                let size = PROC_SIZE as i32;

                for y in 0..PROC_SIZE as i32 {
                    let (y_change, y_change_2) = match y {
                        y if y == size - 1 => (size, (size - 1) * size),
                        0 => (-((size - 1) * size), -size),
                        _ => (size, -size),
                    };

                    for x in 0..PROC_SIZE as i32 {
                        let y_offset_a = (offset as i32 - y_change) as usize;
                        let y_offset_b = (offset as i32 - y_change_2) as usize;
                        
                        let dx = match x {
                            x if x == size - 1 => ptr[offset - 1].wrapping_sub(ptr[offset - (PROC_SIZE - 1)]),
                            0 => ptr[offset + (PROC_SIZE - 1)].wrapping_sub(ptr[offset + 1]),
                            _ => ptr[offset - 1].wrapping_sub(ptr[offset + 1]),
                        } as i32;

                        let dy = ptr[y_offset_a].wrapping_sub(ptr[y_offset_b]) as i32;

                        let x_offset = (x.wrapping_add(dx >> 3) as usize) & (PROC_SIZE - 1);
                        let y_offset = (y.wrapping_add(dy >> 3) as usize) & (PROC_SIZE - 1);

                        let mut light = (NUM_WATER_SHADES as i32 / 2).wrapping_sub(dx.wrapping_shr(lightval as u32));

                        if light > NUM_WATER_SHADES as i32 - 1 {
                            light = NUM_WATER_SHADES as i32 - 1;
                        }
                        
                        if light < 0 {
                            light = 0;
                        }

                        let color = bitmap.data()[y_offset * PROC_SIZE + x_offset];
                        let ci = (color & !OPAQUE_FLAG) as usize;
                        let l = light as usize;

                        dest_bitmap[offset] = WATER_LUT.hi[l][ci >> 8] + WATER_LUT.lo[l][ci & 0xFF] as u16;

                        offset += 1;
                    }
                }
            }
        }

        memory.replace_memory(f, b);
    }
}

impl EmitterEffect for WaterEffect {
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        if context.base_emitter.can_emit(context.src_bitmap.frame_count() + context.src_bitmap.emitters.len()) {
            self.effect.step(context, memory);
        }

        let easter_egg_ref = self.easter_egg_ref.take();

        if easter_egg_ref.is_some() {
            let b = easter_egg_ref.unwrap();

            {
                // When some easter egg is set, we draw it into proc memory
                let easter_bitmap = b.borrow();
                let src = easter_bitmap.data();
                let dst = memory.front_s16();

                let sw = easter_bitmap.width();
                let sh = easter_bitmap.height();
                let x1 = (PROC_SIZE / 2) - (sw / 2);
                let y1 = (PROC_SIZE / 2) - (sh / 2);

                // Make sure size is valid
                if sw <= PROC_SIZE && sh <= PROC_SIZE {
                    for i in 0..sh {
                        for t in 0..sw {
                            if (src[i * sw + t] & OPAQUE_FLAG) > 0 {
                                let off = ((y1 + i) * PROC_SIZE) + t + x1;
                                dst[off] = dst[off].wrapping_add(200)
                            }
                        }
                    }
                }
                else {
                    warn!("Water easter egg source image not correct resolution");
                }
            }

            self.easter_egg_ref.replace(b);
        }

        self.draw_water(
            self.draw_type, 
            context.src_bitmap.base_bitmap_ref.as_ref().expect("need an allocated bitmap16"),
            dest,
            memory);

        let mut thickness = self.thickness as i32;

        if context.src_bitmap.osc_time > 0.0 {
            let start = std::cmp::min(context.src_bitmap.osc_value, self.thickness);
            let end = std::cmp::max(context.src_bitmap.osc_value, self.thickness);
            let diff = (end - start) as i32;

            let ticks = context.src_bitmap.system_clock_ref.get_ticks();

            if diff > 0 {
                let frametime = context.src_bitmap.osc_time / diff as f32;
                let mut current_frametime = ((ticks as i32 / 1000) / frametime.abs().max(1.0) as i32);

                current_frametime %= diff * 2;

                current_frametime = if current_frametime >= diff {
                    (diff - 1) - (current_frametime % diff)
                } else {
                    diff
                };

                thickness = (start as i32 + current_frametime);
            }
        }

        self.calc_water(WaterVariant::V1, thickness, memory);
    }
}