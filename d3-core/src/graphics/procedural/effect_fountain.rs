use super::{effect_fire, place_point, ps_rand, DoubleBufferStorage, EmittedElement, EmitterEffect, BRIGHT_COLOR};

#[derive(Debug, Clone, Default)]
pub struct FountainEffect {
    elements: Vec<EmittedElement>,
}

impl effect_fire::FireEmitterEffect for FountainEffect {
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        let mut rand = crate::create_rng();

        let data = memory.front_8();

        if context.can_emit() {
            let num = (ps_rand(&mut rand) % 4) as usize + 1;

            for _ in 0..num {
                let frames_left: usize;
                let dy: f32;

                if (ps_rand(&mut rand) % 10) == 0 {
                    dy = -( (ps_rand(&mut rand) % 100) as f32 / 300.0 );
                    frames_left = (ps_rand(&mut rand) % 6) as usize + 3;
                }
                else {
                    dy = (ps_rand(&mut rand) % 100) as f32 / 50.0;
                    frames_left = (ps_rand(&mut rand) % 10) as usize + 15;
                }

                let e = EmittedElement {
                    dx: ((ps_rand(&mut rand) % 100) as f32) - 50.0 / 200.0,
                    dy: dy,
                    frames_left: frames_left,
                    speed: context.base_emitter.speed,
                    color: BRIGHT_COLOR,
                    size: 0,
                    x1: context.base_emitter.x1,
                    y1: context.base_emitter.y1,
                };

                self.elements.push(e);
            }
        }

        self.elements.retain_mut(|e| {
            place_point(data, e.x1, e.y1, e.color);

            e.frames_left = e.frames_left.saturating_sub(1);
            e.color = e.color.saturating_sub(1);

            e.frames_left > 0 || e.color > 0
        });

        for e in self.elements.iter_mut() {
            e.x1 += e.dx;
            e.y1 += e.dy;
        }
    }
}