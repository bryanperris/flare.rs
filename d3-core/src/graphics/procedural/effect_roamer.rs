use super::{effect_fire, place_point, ps_rand, DoubleBufferStorage, EmittedElement, EmitterEffect, BRIGHT_COLOR};

#[derive(Debug, Clone, Default)]
pub struct RoamerEffect {
    elements: Vec<EmittedElement>,
    x1: f32,
    y1: f32
}

impl RoamerEffect {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            elements: Vec::new(),
            x1: x,
            y1: y
        }
    }
}

impl effect_fire::FireEmitterEffect for RoamerEffect {
    fn step(&mut self, context: &mut super::Context, memory: &mut DoubleBufferStorage, dest: &mut [u16]) {
        let mut rand = crate::create_rng();

        self.x1 += (ps_rand(&mut rand) % 5) as f32 - 2.0;
        self.y1 += (ps_rand(&mut rand) % 5) as f32 - 2.0;


        if context.can_emit() {
            let num = ps_rand(&mut rand) as usize & 7;

            let x1 = context.base_emitter.x1 + self.x1;
            let y1 = context.base_emitter.y1 + self.y1;

            for i in 0..num {
                let e = EmittedElement {
                    dx: 0.0,
                    dy: 0.0,
                    frames_left: (ps_rand(&mut rand) % 10) as usize + 15,
                    speed: context.base_emitter.speed,
                    color: BRIGHT_COLOR,
                    size: 0,
                    x1: x1,
                    y1: y1,
                };

                self.elements.push(e);
            }

            self.elements.retain_mut(|e| {
                place_point(memory.front_8(), e.x1, e.y1, e.color);
    
                e.frames_left = e.frames_left.wrapping_sub(1);
                e.color = e.color.wrapping_sub(1);
    
                e.frames_left > 0 || e.color > 0
            });
    
            for e in self.elements.iter_mut() {
                let speed_adjust = 1.0 + (e.speed as f32 / 255.0) * 2.0;
    
                let rand_x = (ps_rand(&mut rand) % 3) as f32;
                let rand_y = (ps_rand(&mut rand) % 3) as f32;
    
                let dx = (rand_x - 1.0) * speed_adjust;
                let dy = (rand_y - 1.0) * speed_adjust;
                
                e.x1 += dx;
                e.y1 += dy;
            }
        }
    }
}