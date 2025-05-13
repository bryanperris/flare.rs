use super::{effect_water::WaterEffectVariant, ps_rand, BaseEmitter, DoubleBufferStorage, PROC_SIZE};

#[derive(Debug, Clone, Default)]
pub struct HeightBlobWaterEffect;

impl WaterEffectVariant for HeightBlobWaterEffect {
    fn step(&self, context: &mut super::Context, memory: &mut DoubleBufferStorage) {
        let data = memory.front_s16();

        let radius = context.base_emitter.size as i32;
        let height = context.base_emitter.speed as i16;
        let rquad = radius * radius;

        let x = context.base_emitter.x1.trunc() as i32;
        let y = context.base_emitter.y1.trunc() as i32;

        let mut left = -radius;
        let mut right = radius;
        let mut top = -radius;
        let mut bottom = radius;

        let size = PROC_SIZE as i32;

        // Perform edge clipping
        if x - radius < 1 {
            left -= x - radius - 1;
        }

        if y - radius < 1 {
            top -= y - radius - 1;
        }

        if x + radius > size - 1 {
            right -= x + radius - size + 1;
        }

        if y + radius > size - 1 {
            bottom -= y + radius - size + 1;
        }

        for cy in top..bottom {
            let cyq = cy * cy;

            for cx in left..right {
                if cx * cx + cyq < rquad {
                    let off = PROC_SIZE * (cy + y) as usize + (cx + x) as usize;
                    data[off] = data[off].wrapping_add(height);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SineBlobWaterEffect;

impl WaterEffectVariant for SineBlobWaterEffect {
    fn step(&self, context: &mut super::Context, memory: &mut DoubleBufferStorage) {
        let data = memory.front_s16();

        let radius = context.base_emitter.size as i32;
        let radsquare = radius * radius;
        let height = context.base_emitter.speed;
        let rquad = radius * radius;
        let length = (1024.0 / radius as f32) * (1024.0 / radius as f32);

        let x = context.base_emitter.x1.trunc() as i32;
        let y = context.base_emitter.y1.trunc() as i32;

        let mut left = -radius;
        let mut right = radius;
        let mut top = -radius;
        let mut bottom = radius;

        let size = PROC_SIZE as i32;

        // Perform edge clipping
        if x - radius < 1 {
            left -= x - radius - 1;
        }

        if y - radius < 1 {
            top -= y - radius - 1;
        }

        if x + radius > size - 1 {
            right -= x + radius - size + 1;
        }

        if y + radius > size - 1 {
            bottom -= y + radius - size + 1;
        }

        for cy in top..bottom {
            for cx in left..right {
                let square = cy * cy + cx * cx;

                if square < radsquare {
                    let dist = (square as f32 * length).sqrt();
                    let addval = dist.cos() * height as f32;
                    let addval = addval.trunc().abs() as i32;
                    let addval = addval / 8;
                    let offset = PROC_SIZE * (cy + y) as usize + (cx + x) as usize;
                    data[offset] = data[offset].wrapping_add(addval as i16);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RainDropsWaterEffect;

impl WaterEffectVariant for RainDropsWaterEffect {
    fn step(&self, context: &mut super::Context, memory: &mut DoubleBufferStorage) {
        // TODO: This could be better

        let mut rand = crate::create_rng();

        let prev_freq = context.base_emitter.frequency;
        let prev_size = context.base_emitter.size;
        let prev_speed = context.base_emitter.speed;
        let prev_x1 = context.base_emitter.x1;
        let prev_y1 = context.base_emitter.y1;

        let add_height_effect = HeightBlobWaterEffect::default();

        context.base_emitter.frequency = 0;
        context.base_emitter.size = ((ps_rand(&mut rand) % 3) + 1) as u8;
        context.base_emitter.speed = std::cmp::max(0, (prev_speed as u32).wrapping_add(ps_rand(&mut rand) % 10).wrapping_sub(5) as u8);

        let x1_rand = (ps_rand(&mut rand) as u8 % (prev_size * 2)).wrapping_sub(prev_size);
        let y1_rand = (ps_rand(&mut rand) as u8 % (prev_size * 2)).wrapping_sub(prev_size);

        context.base_emitter.x1 += x1_rand as f32;
        context.base_emitter.y1 += y1_rand as f32;

        add_height_effect.step(context, memory);

        context.base_emitter.x1 = prev_x1;
        context.base_emitter.y1 = prev_y1;
        context.base_emitter.size = prev_size;
        context.base_emitter.speed = prev_speed;
        context.base_emitter.frequency = prev_freq;
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlobDropsWaterEffect;

impl WaterEffectVariant for BlobDropsWaterEffect {
    fn step(&self, context: &mut super::Context, memory: &mut DoubleBufferStorage) {
        // TODO: This could be better

        let mut rand = crate::create_rng();

        let prev_freq = context.base_emitter.frequency;
        let prev_size = context.base_emitter.size;
        let prev_speed = context.base_emitter.speed;
        let prev_x1 = context.base_emitter.x1;
        let prev_y1 = context.base_emitter.y1;

        let add_height_effect = HeightBlobWaterEffect::default();

        context.base_emitter.frequency = 0;
        context.base_emitter.size = ((ps_rand(&mut rand) % 6) + 4) as u8;
        context.base_emitter.speed = std::cmp::max(0, prev_speed.wrapping_add((ps_rand(&mut rand) % 50) as u8).wrapping_sub(25));

        let x1_rand = (ps_rand(&mut rand) as u8 % (prev_size * 2)).wrapping_sub(prev_size);
        let y1_rand = (ps_rand(&mut rand) as u8 % (prev_size * 2)).wrapping_sub(prev_size);

        context.base_emitter.x1 += x1_rand as f32;
        context.base_emitter.y1 += y1_rand as f32;

        add_height_effect.step(context, memory);

        context.base_emitter.x1 = prev_x1;
        context.base_emitter.y1 = prev_y1;
        context.base_emitter.size = prev_size;
        context.base_emitter.speed = prev_speed;
        context.base_emitter.frequency = prev_freq;
    }
}