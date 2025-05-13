use core::{
    cell::{RefCell, RefMut},
    fmt::Debug,
};
use std::{io::Read, rc::Rc, sync::Arc};

use crate::{
    common::SharedMutRef, graphics::OPAQUE_FLAG, math::vector2d::Vector2D, rand::ps_rand, string::D3String
};

use super::{
    bitmap::{Bitmap16, BitmapFlags},
    detail_settings::DetailSettings,
    FrameCounter,
};

// use typed_builder::TypedBuilder;
use derive_builder::Builder;

extern crate tinyrand;
use effect_fire::fire_blit;
use tinyrand::{Rand, StdRand};

use once_cell::sync::Lazy;

pub mod effect_cone;
pub mod effect_fall;
pub mod effect_fire;
pub mod effect_fountain;
pub mod effect_lightning;
pub mod effect_random_ember;
pub mod effect_rising_ember;
pub mod effect_roamer;
pub mod effect_water;
pub mod water_effects;

#[cfg(test)]
pub mod tests;

const BRIGHT_COLOR: u8 = 254;
const TABLE_SIZE: usize = 256;
const TABLE_MASK: usize = TABLE_SIZE - 1;
const PROC_SIZE: usize = 128;
const EMITTER_LIMIT: usize = 10;

const fn generate_default_palette() -> [u16; ProcPalette::SIZE] {
    let mut palette = [0u16; ProcPalette::SIZE];

    /* First half */
    let mut i = 0;
    while i < 128 {
        let fr = i as f32 / 127.0;
        let ib = (fr * 31.0) as u16;
        let ig = (((fr * 16.0) as u32) << 5) as u16;
        palette[i] = OPAQUE_FLAG | ig | ib;
        i += 1;
    }

    /* Second half */
    let mut i = 0;
    while i < 128 {
        let norm = i as f32 / 127.0;
        let ir = (norm * 31.0) as u32;
        let ig = 16 + (norm * 15.0) as u32;
        palette[i + 128] = OPAQUE_FLAG | (ir << 10) as u16 | (ig << 5) as u16 | 0x1F;
        i += 1;
    }

    palette
}

fn lerp(t: f32, x0: f32, x1: f32) -> f32 {
    x0 + t * (x1 - x0)
}

// Used for the represented type
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EmitterType {
    Fire(FireEmitterType),
    Water(WaterEmitterType),
}

// Used for the represented type
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FireEmitterType {
    LineLightning,
    SphereLightning,
    Straight,
    RisingEmber,
    RandomEmbers,
    Spinners,
    Roamers,
    Fountain,
    Cone,
    FallRight,
    FallLeft,
}

// Used for the represented type
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WaterEmitterType {
    HeightBlob,
    SineBlob,
    RainDrops,
    BlobDrops,
}

#[derive(Debug, Clone, PartialEq)]
struct DoubleBufferStorage {
    memory: [Option<Vec<u16>>; 2],
    front: usize,
    back: usize,
}

impl DoubleBufferStorage {
    fn new(width: usize, height: usize) -> Self {
        Self {
            memory: [Some(vec![0; width * height]), Some(vec![0; width * height])],
            front: 0,
            back: 1,
        }
    }

    fn swap(&mut self) {
        let temp = self.front;
        self.front = self.back;
        self.back = temp;
    }

    fn take_memory(&mut self) -> (Vec<u16>, Vec<u16>) {
        (
            self.memory[self.front].take().unwrap(),
            self.memory[self.back].take().unwrap(),
        )
    }

    fn replace_memory(&mut self, front: Vec<u16>, back: Vec<u16>) {
        let _ = self.memory[self.front].replace(front);
        let _ = self.memory[self.back].replace(back);
    }

    fn front_16(&mut self) -> &mut [u16] {
        self.memory[self.front].as_mut().unwrap()
    }

    fn back_16(&mut self) -> &mut [u16] {
        self.memory[self.back].as_mut().unwrap()
    }

    fn front_8(&mut self) -> &mut [u8] {
        // Get a mutable u8 slice
        unsafe {
            let m = self.memory[self.front].as_mut().unwrap();
            std::slice::from_raw_parts_mut(m.as_mut_ptr() as *mut u8, m.len())
        }
    }

    fn back_8(&mut self) -> &mut [u8] {
        // Get a mutable u8 slice
        unsafe {
            let m = self.memory[self.back].as_mut().unwrap();
            std::slice::from_raw_parts_mut(m.as_mut_ptr() as *mut u8, m.len())
        }
    }

    fn front_s16(&mut self) -> &mut [i16] {
        // Get a mutable u8 slice
        unsafe {
            let m = self.memory[self.front].as_mut().unwrap();
            std::slice::from_raw_parts_mut(m.as_mut_ptr() as *mut i16, m.len())
        }
    }

    fn back_s16(&mut self) -> &mut [i16] {
        // Get a mutable u8 slice
        unsafe {
            let m = self.memory[self.back].as_mut().unwrap();
            std::slice::from_raw_parts_mut(m.as_mut_ptr() as *mut i16, m.len())
        }
    }
}

#[derive(Debug, Clone)]
struct BaseEmitter {
    pub effect: Option<Box<dyn EmitterEffect>>,
    pub frequency: usize,
    pub speed: u8,
    pub color: u8,
    pub size: u8,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BaseEmitter {
    pub fn can_emit(&self, frame_count: usize) -> bool {
        self.frequency == 0 || (frame_count % self.frequency) == 0
    }
}

#[derive(Debug, Clone)]
struct EmittedElement {
    pub dx: f32,
    pub dy: f32,
    pub frames_left: usize,
    pub speed: u8,
    pub color: u8,
    pub size: usize,
    pub x1: f32,
    pub y1: f32,
}

struct Context<'e> {
    src_bitmap: &'e mut ProceduralBitmap16,
    base_emitter: &'e mut BaseEmitter,
    gametime: f32,
}

impl<'e> Context<'e> {
    fn can_emit(&self) -> bool {
        self.base_emitter.can_emit(self.src_bitmap.frame_count())
    }
}

trait EmitterEffectClone {
    fn clone_box(&self) -> Box<dyn EmitterEffect>;
}

trait EmitterEffect: core::fmt::Debug + EmitterEffectClone {
    fn step(
        &mut self,
        src_bitmap: &mut Context,
        memory: &mut DoubleBufferStorage,
        dest: &mut [u16],
    );
}

impl<T> EmitterEffectClone for T
where
    T: 'static + EmitterEffect + Clone,
{
    fn clone_box(&self) -> Box<dyn EmitterEffect> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn EmitterEffect> {
    fn clone(&self) -> Box<dyn EmitterEffect> {
        self.clone_box()
    }
}


trait ProceduralModelClone {
    fn clone_box(&self) -> Box<dyn ProceduralModel>;
}
trait ProceduralModel: core::fmt::Debug + ProceduralModelClone {
    fn on_frame_start(
        &self,
        src_bitmap: &mut ProceduralBitmap16,
        memory: &mut DoubleBufferStorage,
        dest: &mut [u16],
    );
    fn on_frame_end(
        &self,
        src_bitmap: &mut ProceduralBitmap16,
        memory: &mut DoubleBufferStorage,
        dest: &mut [u16],
    );
}

impl<T> ProceduralModelClone for T
where
    T: 'static + ProceduralModel + Clone,
{
    fn clone_box(&self) -> Box<dyn ProceduralModel> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ProceduralModel> {
    fn clone(&self) -> Box<dyn ProceduralModel> {
        self.clone_box()
    }
}

fn place_point(data: &mut [u8], x: f32, y: f32, color: u8) {
    let x = (x as usize) & (PROC_SIZE - 1);
    let y = (y as usize) & (PROC_SIZE - 1);
    data[y * PROC_SIZE + x] = color;
}

#[derive(Debug, Builder, Clone)]
#[builder(pattern = "owned")]
pub struct ProceduralBitmap16 {
    #[builder(setter(into))]
    name: D3String,

    detail_settings_ref: SharedMutRef<DetailSettings>,
    frame_counter_ref: FrameCounter,
    system_clock_ref: Arc<dyn crate::common::SystemClock>,

    // The memory effects can draw into
    #[builder(default=Some(DoubleBufferStorage::new(PROC_SIZE, PROC_SIZE)), setter(skip))]
    memory: Option<DoubleBufferStorage>,

    // Optional source bitmap image for blending effects with
    #[builder(default, setter(strip_option))]
    base_bitmap_ref: Option<SharedMutRef<dyn Bitmap16>>,

    // The destination bitmap image
    #[builder(setter(custom))]
    dest_bitmap: Option<Vec<u16>>,

    #[builder(default, setter(strip_option))]
    model: Option<Box<dyn ProceduralModel>>,

    #[builder(default=ProcPalette::DEFAULT)]
    palette: ProcPalette,

    #[builder(default, setter(skip))]
    emitters: Vec<BaseEmitter>,

    // Related to fire emitters
    #[builder(default=128)]
    heat: u8,

    #[builder(default)]
    osc_time: f32,

    #[builder(default=8)]
    osc_value: u8,
}

impl ProceduralBitmap16Builder {
    fn dest_bitmap(mut self, width: usize, height: usize) -> Self {
        self.dest_bitmap = Some(Some(vec![0u16; width * height]));
        self
    }
}

impl ProceduralBitmap16 {
    pub fn append_emitters(&mut self, emitters: &mut Vec<BaseEmitter>) {
        self.emitters.extend(emitters.drain(..));
    }

    pub fn append_emitter(&mut self, emitter: BaseEmitter) {
        self.emitters.push(emitter);
    }

    pub fn clear_emitters(&mut self) {
        self.emitters.clear();
    }

    pub fn frame_count(&self) -> usize {
        self.frame_counter_ref.load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_ticks(&self) -> u128 {
        self.system_clock_ref.get_ticks()
    }

    pub fn is_procedurals_enabled(&self) -> bool {
        self.detail_settings_ref.borrow().is_procedurals_enabled()
    }

    pub fn base_bitmap(&self) -> Option<core::cell::Ref<'_, dyn Bitmap16>> {
        if self.base_bitmap_ref.is_some() {
            Some(self.base_bitmap_ref.as_ref().unwrap().borrow())
        } else {
            None
        }
    }

    pub fn step(&mut self, gametime: f32) {
        {
            let bitmap = self.base_bitmap_ref.as_ref().unwrap().borrow();

            if bitmap.width() != PROC_SIZE {
                error!(
                    "Couldn't evaluate procedural because its not {} x {}",
                    PROC_SIZE, PROC_SIZE
                );
                return;
            }
        }

        let mut emitters = std::mem::take(&mut self.emitters);
        let mut mem = self.memory.take().unwrap();
        let mut dest = self.dest_bitmap.take().unwrap();
        let model = self.model.take();

        // Execute the pre frame event
        if let Some(ref m) = model {
            m.on_frame_start(self, &mut mem, &mut dest);
        }

        for e in emitters.iter_mut() {
            if let Some(mut effect) = e.effect.take() {
                let mut context = Context {
                    src_bitmap: self,
                    base_emitter: e,
                    gametime: gametime,
                };

                effect
                    .as_mut()
                    .step(&mut context, &mut mem, dest.as_mut_slice());

                e.effect = Some(effect);
            }
        }

        // Execute frame end event
        if let Some(ref m) = model {
            m.on_frame_end(self, &mut mem, &mut dest);
        }

        mem.swap();

        self.memory = Some(mem);
        self.dest_bitmap = Some(dest);
        self.model = model;
        std::mem::replace::<Vec<BaseEmitter>>(&mut self.emitters, emitters);
    }
}

impl Bitmap16 for ProceduralBitmap16 {
    fn data(&self) -> &[u16] {
        &self.dest_bitmap.as_ref().unwrap().as_slice()
    }

    fn width(&self) -> usize {
        PROC_SIZE
    }

    fn height(&self) -> usize {
        PROC_SIZE
    }

    fn mip_levels(&self) -> usize {
        0
    }

    fn flags(&self) -> &super::bitmap::BitmapFlags {
        &BitmapFlags::None
    }

    fn name(&self) -> &crate::string::D3String {
        &self.name
    }

    fn format(&self) -> super::bitmap::BitmapFormat {
        super::bitmap::BitmapFormat::Fmt1555
    }

    fn make_funny(&mut self) {
        todo!()
    }
}
#[derive(Debug)]
pub struct ProceduralCommon {
    perm: [u8; TABLE_SIZE],
    noise: [f32; TABLE_SIZE * 3],
    fade: [u16; 32768],
}

static COMMON: Lazy<ProceduralCommon> = Lazy::new(|| {
    let mut perm = [0u8; TABLE_SIZE];

    let mut noise = [0.0f32; TABLE_SIZE * 3];

    /* Init the noise */
    let mut rand = crate::create_rng();

    for i in 0..TABLE_SIZE {
        let r = perm[i] = ps_rand(&mut rand) as u8;
        let random_float: f32 = ps_rand(&mut rand) as f32 / std::i16::MAX as f32;
        let z = 1.0 - 2.0 * random_float;

        /* r is radius of x,y circle */
        let r = (1.0 - z * z).sqrt();

        /* theta is angle in (x,y) */
        let random_float: f32 = ps_rand(&mut rand) as f32 / std::i16::MAX as f32;
        let theta = 2.0 * 3.14 * random_float;

        noise[i + 0] = r * theta.cos();
        noise[i + 1] = r * theta.sin();
        noise[i + 2] = z;
    }

    /* Initialize the fade table */
    let mut fade_table = [0u16; 32768];

    for i in 0..fade_table.len() {
        let r = (i >> 10) & 0x1F;
        let g = (i >> 5) & 0x1F;
        let b = i & 0x1F;

        let r = 0.max(r - 1);
        let g = 0.max(g - 1);
        let b = 0.max(b - 1);

        fade_table[i] = OPAQUE_FLAG | (r << 10) as u16 | (g << 5) as u16 | b as u16;
    }

    ProceduralCommon {
        noise: noise,
        perm: perm,
        fade: fade_table,
    }
});

impl ProceduralCommon {
    // TODO: These can be in a different spot
    // const NAMES: [&str; 11] = [
    //     "Line Lightning",
    //     "Sphere lightning",
    //     "Straight",
    //     "Rising Embers",
    //     "Random Embers",
    //     "Spinners",
    //     "Roamers",
    //     "Fountain",
    //     "Cone",
    //     "Fall Right",
    //     "Fall Left",
    // ];

    // const WATER_PROC_NAMES: [&str; 4] = [
    //     "Height blob",
    //     "Sine Blob",
    //     "Random Raindrops",
    //     "Random Blobdrops",
    // ];

    fn perm(&self, x: usize) -> u8 {
        self.perm[x & 0xFF]
    }

    fn index(&self, x: usize, y: usize) -> usize {
        self.perm(x) as usize + self.perm(y) as usize
    }

    /// Gets a lattice point for our noise
    fn grad_lattice(&self, x: usize, y: usize, fx: f32, fy: f32) -> f32 {
        let i = self.index(x, y);
        self.noise[i] * fx + self.noise[i + 1] * fy
    }

    fn grad_noise(&self, x: f32, y: f32) -> f32 {
        let ix = x.floor() as usize;
        let fx0 = x - ix as f32;
        let fx1 = fx0 - 1.0;
        let wx = fx0; // smoothstep
        let iy = y.floor().trunc() as usize;
        let fy0 = y - iy as f32;
        let fy1 = fy0 - 1.0;
        let wy = fy0; // smoothstep
        let vx0 = self.grad_lattice(ix, iy, fx0, fy0);
        let vx1 = self.grad_lattice(ix + 1, iy, fx1, fy0);
        let vy0 = lerp(wx, vx0, vx1);
        let vx0 = self.grad_lattice(ix, iy + 1, fx0, fy1);
        let vx1 = self.grad_lattice(ix + 1, iy + 1, fx1, fy1);
        let vy1 = lerp(wx, vx0, vx1);

        lerp(wy, vy0, vy1)
    }
}

#[derive(Debug, Clone)]
pub struct ProcPalette {
    table: [u16; ProcPalette::SIZE],
}

impl ProcPalette {
    pub const SIZE: usize = 256;
    pub const DEFAULT: ProcPalette = ProcPalette {
        table: generate_default_palette(),
    };

    pub fn from_raw(table: [u16; ProcPalette::SIZE]) -> Self {
        Self { table: table }
    }

    pub fn new(r: &[u8; Self::SIZE], g: &[u8; Self::SIZE], b: &[u8; Self::SIZE]) -> Self {
        let mut table = [0; Self::SIZE];

        for i in 0..Self::SIZE {
            let mut r = r[i] as f32 / 255.0;
            let mut g = g[i] as f32 / 255.0;
            let mut b = b[i] as f32 / 255.0;

            r *= 31.0;
            g *= 31.0;
            b *= 31.0;

            let r = ((r.trunc() as i32) << 10) as u16;
            let g = ((g.trunc() as i32) << 5) as u16;
            let b = b.trunc() as u16;

            table[i] = OPAQUE_FLAG | r | g | b;
        }

        Self { table: table }
    }

    pub fn table(&self) -> &[u16] {
        &self.table[0..]
    }
}
