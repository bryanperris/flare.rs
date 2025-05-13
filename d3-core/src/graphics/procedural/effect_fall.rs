use super::{effect_fire, place_point, ps_rand, DoubleBufferStorage, EmittedElement, EmitterEffect, BRIGHT_COLOR};

pub const LEFT: u8 = 0;
pub const RIGHT: u8 = 1;

#[derive(Debug, Clone, Default)]
pub struct FallEffect<const D: u8> {
    elements: Vec<EmittedElement>,
}

impl<const D: u8> effect_fire::FireEmitterEffect for FallEffect<D> {
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        let mut rand = crate::create_rng();

        if context.can_emit() {
            let num = (ps_rand(&mut rand) % 2) as usize + 1;

            let x1 = (ps_rand(&mut rand) % 5).wrapping_sub(2);
            let y1 = (ps_rand(&mut rand) % 5).wrapping_sub(2);

            let dx;

            match D {
                0 => dx = -1.0,
                1 => dx = 1.0,
                _ => panic!("invalid direction")
            }

            for _ in 0..num {
                let e = EmittedElement {
                    dx: dx,
                    dy: -( (ps_rand(&mut rand) % 100) as f32 / 300.0 ),
                    frames_left: (ps_rand(&mut rand) % 15) as usize + 25,
                    speed: 0,
                    color: BRIGHT_COLOR,
                    size: 0,
                    x1: context.base_emitter.x1 + x1 as f32,
                    y1: context.base_emitter.y1 + y1 as f32
                };

                self.elements.push(e);
            }
        }

        self.elements.retain_mut(|e| {
            place_point(memory.front_8(), e.x1, e.y1, e.color);

            e.frames_left = e.frames_left.saturating_sub(1);
            e.color = e.color.saturating_sub(1);

            e.frames_left > 0 || e.color > 0
        });

        for e in self.elements.iter_mut() {
            let dx_delta = (ps_rand(&mut rand) % 100) as f32 / 2000.0;

            if e.dx.trunc() > 0.0 {
                match D {
                    0 => e.dx += dx_delta,
                    1 => e.dx -= dx_delta,
                    _ => panic!("invalid direction")
                }
            }

            if e.dy.trunc() < 2.0 {
                e.dy += (ps_rand(&mut rand) % 100) as f32 / 1000.0;
            }

            e.x1 += e.dx;
            e.y1 += e.dy;
        }
    }
}