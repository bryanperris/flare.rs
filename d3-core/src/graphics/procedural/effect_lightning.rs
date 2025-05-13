use core::task::Context;

use crate::{game::context, graphics::procedural::PROC_SIZE, math::vector2d::Vector2D};

use super::{effect_fire, ps_rand, BaseEmitter, DoubleBufferStorage, EmitterEffect, BRIGHT_COLOR};

#[derive(Debug, Clone, Default)]
pub struct LightningEffect;

#[derive(Debug, Clone, Default)]
pub struct SphereLightningEffect;

fn draw_line(data: &mut [u8], x1: f32, y1: f32, x2: f32, y2: f32, color: u8) {
    let mut data_offset = 0usize;

    let x_mask = PROC_SIZE - 1;
    let y_mask = PROC_SIZE - 1;

    let mut xinc = true; let mut yinc = true;

    struct Rect {
        x1: f32, x2: f32,
        y1: f32, y2: f32
    }

    // Check to see if our x coords are reversed
    let coords = if x1 > x2 {
        Rect {
            x1: x2,
            x2: x1,
            y1: y2,
            y2: y1
        }
    } else {
        Rect {
            x1: x1,
            x2: x2,
            y1: y1,
            y2: y2
        }
    };

    let mut dx = coords.x2 - coords.x1;
    let mut dy = coords.y2 - coords.y1;

    if dx < 0.0 {
        xinc = false;
        dx = -dx;
    }

    if dy < 0.0 {
        yinc = false;
        dy = -dy;
    }

    let mut x = coords.x1 as usize & x_mask;
    let mut y = coords.y1 as usize & y_mask;
    data_offset += y * PROC_SIZE;

    // X is greater than y
    if dx >= dy {
        let mut error_term = 0.0;

        for i in 0..dx as usize {
            data[data_offset + x] = color;

            x = if xinc { x + 1 } else { x - 1 };
            x &= x_mask;

            error_term += dy;

            if error_term >= dx {
                y = if yinc { y.wrapping_sub(1) } else { y.wrapping_sub(1) };
                y &= y_mask;
                data_offset = y * PROC_SIZE;
                error_term -= dx;
            }
        }
    } else {
        let mut error_term = 0.0;

        for i in 0..dy as usize {
            data[data_offset + x] = color;

            y = if yinc { y.wrapping_sub(1) } else { y.wrapping_sub(1) };
            y &= y_mask;

            error_term += dx;
            data_offset = y * PROC_SIZE;

            if error_term >= dy {
                x = if xinc { x + 1 } else { x - 1 };
                x &= x_mask;
                error_term -= dy;
            }
        }
    };
}

fn add_lightning(x2: f32, y2: f32, color: u8, base_emitter: &BaseEmitter, data: &mut [u8]) {
    let mut delta = Vector2D {
        x: x2 - base_emitter.x1,
        y: y2 - base_emitter.y1
    };

    let mag = Vector2D::magnitude(&delta);

    if mag < 1.0 {
        return;
    }

    let num_segments = (mag / 8.0).trunc() as usize;

    delta.x /= mag;
    delta.y /= mag;

    let mut current_x = base_emitter.x1 as f32; let mut current_y = base_emitter.y1 as f32;
    let mut from_x = current_x; let mut from_y = current_y;
    let mut rand = crate::create_rng();

    for i in 0..num_segments {
        let mut to_x = current_x + (delta.x * 8.0);
        let mut to_y = current_y + (delta.y * 8.0);

        if i != num_segments - 1 {
            let speed = (base_emitter.speed + 1) as f32;

            let r1 = ps_rand(&mut rand) % 200;
            let r2 = ps_rand(&mut rand) % 200;
            let r1 = r1 as f32 - 100.0;
            let r2 = r2 as f32 - 100.0;

            to_x += delta.x * speed * (r1 / 18.0);
            to_y += delta.y * speed * (r2 / 18.0);
        }

        draw_line(data, from_x, from_y, to_x, to_y, color);

        from_x = to_x;
        from_y = to_y;

        current_x += delta.x * 8.0;
        current_y += delta.y * 8.0;
    }
}

impl effect_fire::FireEmitterEffect for LightningEffect {
    fn step(&mut self, context: &mut super::Context<'_>, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        add_lightning(context.base_emitter.x2, context.base_emitter.y2, context.base_emitter.color, context.base_emitter, memory.front_8());
    }
}

impl effect_fire::FireEmitterEffect for SphereLightningEffect {
    fn step(&mut self, context: &mut super::Context<'_>, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        let settings = context.src_bitmap.detail_settings_ref.borrow();

        if settings.is_procedurals_enabled() && !context.can_emit() {
            return;
        }

        let norm = context.base_emitter.size as f32 / 255.0;
        let len = (norm * PROC_SIZE as f32) / 2.0;

        let mut rand = crate::create_rng();
        let dir = ps_rand(&mut rand) * 2;

        let cos = (dir as f32).cos() * len;
        let sin = (dir as f32).sin() * len;

        let dest_x = context.base_emitter.x1 + cos;
        let dest_y = context.base_emitter.y1 + sin;

        add_lightning(dest_x, dest_y, BRIGHT_COLOR, context.base_emitter, memory.front_8());
    }
}