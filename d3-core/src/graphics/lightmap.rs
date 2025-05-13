use core::cmp::max;

use super::GpuMemoryResource;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LightMapFlags: u8 {
        const None  =        0b00000000;
        /// This lightmap has a specific area that has changed since last frame
        const Limits =  0b00000001;
        /// This lightmap should be drawn with wrapping (not clamping)
        const Wrap =      0b00000010;
    }
}

#[derive(Debug, Clone)]
pub struct LightMap16 {
    width: usize,
    height: usize,
    data: Vec<u16>,
    is_updated: bool,
    flags: LightMapFlags,
    square_resolution: usize,
    x1_delta: u8,
    y1_delta: u8,
    x2_delta: u8,
    y2_delta: u8
}

impl LightMap16 {
    pub fn new(data: &[u16], width: usize, height: usize) -> Self {
        let mut lightmap = LightMap16 {
            width: width,
            height: height,
            data: data.to_vec(),
            is_updated: true,
            square_resolution: 0,
            x1_delta: 0,
            x2_delta: 0,
            y1_delta: 0,
            y2_delta: 0,
            flags: LightMapFlags::None
        };

        // Figure out square size
        // Find power of 2 number

        let res = max(width, height);
        let mut lightmap_res = 2;

        for i in 0..7 {
            let low_num = 1 << i;
            let hi_num = 2 << i;

            if res <= hi_num && res > low_num {
                lightmap_res = hi_num;
                break;
            }
        }

        assert!(lightmap_res >= 2 && lightmap_res <= 128);
        
        lightmap.square_resolution = lightmap_res;

        lightmap
    }

    pub fn set_square_resolution(&mut self, resolution: usize) -> &mut Self {
        self.is_updated = true;
        self.square_resolution = resolution;
        self
    }

    pub fn set_deltas(&mut self, x1: u8, y1: u8, x2: u8, y2: u8) -> &mut Self {
        self.is_updated = true;
        self.x1_delta = x1;
        self.x2_delta = x2;
        self.y1_delta = y1;
        self.y2_delta = y2;
        self
    }

    pub fn set_flags(&mut self, flags: LightMapFlags) -> &mut Self {
        self.is_updated = true;
        self.flags = flags;
        self
    }

    pub fn flags(&self) -> LightMapFlags {
        self.flags
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn data(&self) -> &[u16] {
        self.data.as_slice()
    }

    pub fn data_mut(&mut self) -> &mut [u16] {
        self.is_updated = true;
        &mut self.data
    }
}

impl GpuMemoryResource for LightMap16 {
    fn mark_updated(&mut self) {
        self.is_updated = true;
    }

    fn is_updated(&self) -> bool {
        self.is_updated
    }
}