use super::{DoubleBufferStorage, EmitterEffect, ProceduralModel, PROC_SIZE};

pub struct FireEffectModel {

}

pub fn fade(data: &mut [u8], heat: u8) {
    let fade = ((255 - heat) >> 3) + 1;
    let fade = fade as i32;

    let mut data_offset = 0usize;

    for i in 0..data.len() {
        let pix = data[data_offset] as u8 as i32;

        if pix != 0 {
            if pix - fade <= 0 {
                data[data_offset] = 0;
            } else {
                data[data_offset] = (pix - fade) as u8;
            }
        }

        data_offset += 1;
    }
}

/// Fades and entire bitmap one step closer to black
pub fn blend(memory: &mut DoubleBufferStorage) {
    let (mut f, mut b) = memory.take_memory();

    let src;
    let dst;

    unsafe {
        src = std::slice::from_raw_parts_mut(
            f.as_mut_ptr() as *mut u8, f.len()
        );

        dst = std::slice::from_raw_parts_mut(
            b.as_mut_ptr() as *mut u8, b.len()
        );
    }

    let mut src_offset = 0usize;
    let mut dst_offset = 0usize;

    for i in 0..PROC_SIZE {
        let start_row = src_offset;

        // Get row underneigth
        let mut downrow = if i != PROC_SIZE - 1 {
            src_offset + PROC_SIZE
        } else {
            src_offset
        };

        for t in 0..PROC_SIZE {
            // Get Center
            let mut total = src[src_offset] as usize;

            // Get Right
            total += if t != PROC_SIZE - 1 {
                src[src_offset + 1]
            } else {
                src[start_row]
            } as usize;

            // Get Left
            total += if t > 0 {
                src[src_offset - 1]
            } else {
                src[start_row + PROC_SIZE - 1]
            } as usize;

            // Get Below
            total += src[downrow] as usize;
            total >>= 2;
            dst[dst_offset] = total as u8;

            src_offset += 1;
            dst_offset += 1;
            downrow += 1;
        }
    }

    memory.replace_memory(f, b);
}

pub fn fire_blit(memory: &mut DoubleBufferStorage, dest: &mut [u16], palette: &[u16]) {
    blend(memory);

    let (f, b) = memory.take_memory();

    let back;

    unsafe {
        back = std::slice::from_raw_parts(
            b.as_ptr() as *const u8, b.len()
        );
    }

    for i in 0..b.len() {
        dest[i] = palette[back[i] as usize]
    }

    memory.replace_memory(f, b);
}

#[derive(Debug, Clone)]
pub struct FireEffect {
    pub effect: Box<dyn FireEmitterEffect>,
}

impl EmitterEffect for FireEffect {
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        self.effect.step(context, memory, dest);
    }
}

trait FireEmitterEffectClone {
    fn clone_box(&self) -> Box<dyn FireEmitterEffect>;
}

pub trait FireEmitterEffect: core::fmt::Debug + FireEmitterEffectClone{
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]);
}

impl<T> FireEmitterEffectClone for T
where
    T: 'static + FireEmitterEffect + Clone,
{
    fn clone_box(&self) -> Box<dyn FireEmitterEffect> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn FireEmitterEffect> {
    fn clone(&self) -> Box<dyn FireEmitterEffect> {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct FireModel;
impl ProceduralModel for FireModel {
    fn on_frame_start(&self, src_bitmap: &mut super::ProceduralBitmap16, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        fade(memory.front_8(), src_bitmap.heat);
    }

    fn on_frame_end(&self, src_bitmap: &mut super::ProceduralBitmap16, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        fire_blit(memory, dest, src_bitmap.palette.table());
    }
}