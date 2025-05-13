use core::borrow::Borrow;
use std::collections::btree_map::Values;

use angle::{Angle, EulerAngle};
use blake3::Hash;
use byteorder::{LittleEndian, WriteBytesExt};
use matrix::Matrix;
use vector::Vector;

use crate::{
    gr_color_blue, gr_color_green, gr_color_red, gr_rgb, gr_rgb16, graphics::{
        bitmap::{self, Bitmap16}, color_conversion::{convert_1555_to_grayscale, convert_4444_to_grayscale}, ddgr_color, lightmap::{LightMap16, LightMapFlags}, GpuMemoryResource, GR_RED, OPAQUE_FLAG
    }
};

use super::{node::Node, prelude::*};

const DEFAULT_TEXTURE_DISTANCE: usize = 9999;
pub const TERRAIN_WIDTH: usize = 256;
pub const TERRAIN_DEPTH: usize = 256;
pub const TERRAIN_SIZE: f32 = 16.0;
const TERRAIN_HEIGHT_INCREMENT: f32 = 350.0 / 255.0;

// Forces lod engine not to work for a particular cell
const SHUTOFF_LOD_DELTA: f32 = 800000.0;

// This LOD is totally invisible
const SHUTOFF_LOD_INVISIBLE: f32 = 900000.0;

const MAX_HORIZON_PIECES: usize = 16;

const MAX_STARS: usize = 600;

const MAX_SATELLITES: usize = 5;

const TERRAIN_TEX_WIDTH: usize = 32;

pub const MAX_TERRAIN_HEIGHT: f32 = 350.0;

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct TerrainFlags: u32 {
        const NONE = 0;
        /// Dynamic terrain segment.
        const DYNAMIC = 0b00000001;
        /// Special water segment.
        const SPECIAL_WATER = 0b00000100;
        /// This segment has a mine attached to it.
        const SPECIAL_MINE = 0b00001000;
        /// This segment is invisible.
        const INVISIBLE = 0b00010000;
        /// Region mask that combines several region-specific flags.
        const REGION_MASK = 0b00100000 | 0b01000000 | 0b10000000;
        // NOTE: 32 64 and 128 are reserved for AI stuff  (terrain region partitioning)
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags representing various sky features.
    pub struct SkyFlags: u32 {
        /// No flags set.
        const NONE = 0b00000;
        /// Whether or not the terrain is starred.
        const STARS = 0b00001;
        /// Draw satellites or not.
        const SATELLITES = 0b00010;
        /// Draw fog or not.
        const FOG = 0b00100;
        /// Rotate stars or not.
        const ROTATE_STARS = 0b01000;
        /// Rotate sky or not.
        const ROTATE_SKY = 0b10000;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags representing various satellite features.
    pub struct SatelliteFlags: u32 {
        /// No flags set.
        const NONE = 0b00000;
        /// Draw halo or not.
        const HALO = 0b00001;
        /// Draw atmosphere or not.
        const ATMOSPHERE = 0b00010;
    }
}

// Terrain cells are on a fixed grid so they have no x and z positions.  If you want the x and z
// positions you must calculate them yourself: gridx*TERRAIN_SIZE and gridz*TERRAIN_SIZE

#[derive(Debug, Clone)]
pub struct TerrainSegment {
    /// Y position of the lower left corner of the terrain cell
    pub y: f32,

    /// scalar version of y, it avoid constant conversion between floats and scalars..
    /// TODO: Want to improve this... so we don't need to do this
    pub y_scalar: u8,

    /// The modified y position of this cell - used for LOD
    pub y_modified: f32,

    pub l: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,

    pub object_ref: Option<SharedMutRef<Object>>,
    pub texture_segment_index: usize,

    pub flags: TerrainFlags,
    pub lightmap_quad: usize,
}

impl Default for TerrainSegment {
    fn default() -> Self {
        Self {
            y: Default::default(),
            y_modified: Default::default(),
            l: Default::default(),
            r: Default::default(),
            g: Default::default(),
            b: Default::default(),
            object_ref: Default::default(),
            texture_segment_index: Default::default(),
            flags: TerrainFlags::NONE,
            lightmap_quad: Default::default(),
            y_scalar: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainTextureSegment {
    pub rotation: u8,
    pub tex_index: Option<usize>,
}

impl Default for TerrainTextureSegment {
    fn default() -> Self {
        Self {
            rotation: Default::default(),
            tex_index: None,
        }
    }
}

/// Data for LOD shutoff code
#[derive(Debug, Clone)]
pub struct LodShutoff {
    pub cellnum: usize,
    pub save_delta: Vec<f32>,
}

impl Default for LodShutoff {
    fn default() -> Self {
        Self {
            cellnum: Default::default(),
            save_delta: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Horizon {
    // The two subscripts correspond to the top, middle, and bottom of the horizon piece
    pub vectors: [[Vector; 6]; 16],
    pub u: [[f32; 5]; 16],
    pub v: [[f32; 5]; 16],
    pub color: ddgr_color,
}

impl Default for Horizon {
    fn default() -> Self {
        Self {
            vectors: Default::default(),
            u: Default::default(),
            v: Default::default(),
            color: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Satellite {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub vector: Vector,
    pub flags: SatelliteFlags,
    pub size: f32,
    pub texture: Option<usize>,
}

impl Default for Satellite {
    fn default() -> Self {
        Self {
            r: Default::default(),
            g: Default::default(),
            b: Default::default(),
            vector: Default::default(),
            flags: SatelliteFlags::NONE,
            size: Default::default(),
            texture: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Star {
    vector: Vector,
    color: ddgr_color,
}

impl Default for Star {
    fn default() -> Self {
        Self {
            vector: Default::default(),
            color: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainSky {
    /// true = use texture
    /// false = use gouraud shading
    pub is_textured: bool,

    pub horizon: Horizon,

    pub dome_texture: (),

    pub radius: f32,
    pub rotate_rate: f32,

    pub sky_color: ddgr_color,
    pub fog_color: ddgr_color,

    pub satellites: Vec<Satellite>,

    pub stars: Vec<Star>,

    pub light_source: Vector,
    pub light_angle: Angle,

    pub damage_per_second: f32,
    pub fog_scalar: f32,

    pub flags: SkyFlags,
}

impl Default for TerrainSky {
    fn default() -> Self {
        Self {
            is_textured: Default::default(),
            horizon: Default::default(),
            dome_texture: Default::default(),
            radius: Default::default(),
            rotate_rate: Default::default(),
            sky_color: Default::default(),
            fog_color: Default::default(),
            satellites: vec![Default::default(); MAX_SATELLITES],
            stars: Default::default(),
            light_source: Default::default(),
            light_angle: Default::default(),
            damage_per_second: Default::default(),
            fog_scalar: Default::default(),
            flags: SkyFlags::NONE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkTile {
    pub mine_seg: i32,
    pub mine_sid: i32,
    pub portal_num: i32,
    pub terrain_seg: i32,
}

impl Default for LinkTile {
    fn default() -> Self {
        Self {
            mine_seg: Default::default(),
            mine_sid: Default::default(),
            portal_num: Default::default(),
            terrain_seg: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainMineList {
    pub terrain_seg: i32,
    pub mine_segs: Vec<()>,
}

impl Default for TerrainMineList {
    fn default() -> Self {
        Self {
            terrain_seg: Default::default(),
            mine_segs: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TerrainNormalPair {
    upper_left_triangle: Vector,
    lower_right_triangle: Vector,
}

impl Default for TerrainNormalPair {
    fn default() -> Self {
        Self {
            upper_left_triangle: Default::default(),
            lower_right_triangle: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EdgeRect {
    pub top: u16,
    pub top_count: usize,
    pub left: u16,
    pub left_count: usize,
    pub right: u16,
    pub right_count: usize,
    pub bottom: u16,
    pub bottom_count: usize,
}

impl Default for EdgeRect {
    fn default() -> Self {
        Self {
            top: Default::default(),
            top_count: Default::default(),
            left: Default::default(),
            left_count: Default::default(),
            right: Default::default(),
            right_count: Default::default(),
            bottom: Default::default(),
            bottom_count: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LodMode {
    /// 16x16
    Mode0,
    /// 8x8
    Mode1,
    /// 4x4
    Mode2,
    /// 2x2
    Mode3,
    /// 1x1
    Mode4,
}

#[derive(Debug, Copy, Clone)]
pub struct TerrainRenderInfo {
    pub z: f32,
    // for fixing tjoint problems
    pub edge: EdgeRect,
    pub segment: (),
    pub lod: LodMode,
}

impl Default for TerrainRenderInfo {
    fn default() -> Self {
        Self {
            z: Default::default(),
            edge: Default::default(),
            segment: Default::default(),
            lod: LodMode::Mode4,
        }
    }
}

const MAX_LOD: usize = 4;

#[derive(Debug, Copy, Clone)]
pub struct TerrainClipRect {
    pub top: f32,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for TerrainClipRect {
    fn default() -> Self {
        Self {
            top: Default::default(),
            left: Default::default(),
            right: Default::default(),
            bottom: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TerrainSearch {
    pub on: i32,
    pub found_type: i32,
    pub x: i32,
    pub y: i32,
    pub seg: i32,
    pub face: i32,
}

impl Default for TerrainSearch {
    fn default() -> Self {
        Self {
            on: Default::default(),
            found_type: Default::default(),
            x: Default::default(),
            y: Default::default(),
            seg: Default::default(),
            face: Default::default(),
        }
    }
}

#[derive(Debug, Clone, GameType)]
pub struct Terrain {
    pub checkum: Option<Hash>,
    pub check_portal: i32,
    pub last_drawn: f32,
    pub trans_count: usize,
    pub total_depth: usize,
    pub frame_count: usize,

    pub segments: Vec<TerrainSegment>,
    pub node_lists: Vec<SharedMutRef<Vec<Node>>>,

    // Occlusion data for knowing what to draw
    pub occlusion_map: [[u8; 32]; 256],
    pub occlusion_checksum: i32,

    // Our lighting maps for the terrain, one for each quadrant (starting at lower left)
    pub ligtmaps: [SharedMutRef<LightMap16>; 4],
    pub edge_test: [[i32; 16]; MAX_LOD],
    pub render_info_list: Vec<TerrainRenderInfo>,
    pub visible_z: f32,
    pub average_height: f32,
    pub clip_scale: TerrainClipRect,
    pub from_mine: u8,
    pub tex_segments: Vec<TerrainTextureSegment>,
    pub dynamic_light_table: Vec<u8>,
    pub normals: [Vec<TerrainNormalPair>; 4],
    pub delta_blocks: [Vec<f32>; 4],

    // first object to render after cell has been rendered (only used for SW renderer)
    // TODO? seg_render_obj
    pub sky: TerrainSky,

    // TODO: Editor stuff but won't be here
    pub lod_engine_offset: i32,
    pub texture_distance: f32,

    pub join_map: Vec<u8>,
    pub max_heights: [Vec<i32>; 7],
    pub min_heights: [Vec<i32>; 7],
    pub fast: u8,
    pub flat: u8,
    pub show_invisible: bool,
    pub camera_direction: i32,
    pub sort_direction: i32,
    pub rotate_list: Vec<u16>,
    pub world_point_buffer: Vec<()>, // implement g3Point type,

    pub search: TerrainSearch,
}

impl Default for Terrain {
    fn default() -> Self {
        let mut terrain = Self {
            rotate_list: vec![0; TERRAIN_WIDTH * TERRAIN_DEPTH],
            world_point_buffer: vec![(); TERRAIN_WIDTH * TERRAIN_DEPTH],
            join_map: vec![0; TERRAIN_WIDTH * TERRAIN_DEPTH],
            node_lists: vec![new_shared_mut_ref(Vec::new()); 8],
            ..Default::default()
        };

        for i in 0..TERRAIN_DEPTH {
            for t in 0..TERRAIN_WIDTH {
                let segment = TerrainSegment {
                    flags: TerrainFlags::NONE,
                    lightmap_quad: ((i / 128) * 2) + (t / 128),
                    texture_segment_index: ((i >> 3) * TERRAIN_TEX_WIDTH) + (t >> 3),
                    ..Default::default()
                };
                
                let offset = i * TERRAIN_WIDTH + t;

                terrain.segments[offset] = segment;
                terrain.dynamic_light_table[offset] = 0xFF;
                terrain.tex_segments[offset] = TerrainTextureSegment {
                    rotation: 1 << 4,
                    ..Default::default()
                }
            }
        }

        terrain.checkum = None;

        terrain.init_min_max();
        terrain.init_normals();

        #[cfg(not(feature = "dedicated_server"))]
        for i in 0..MAX_LOD - 1 {
            let w = TERRAIN_WIDTH >> ((MAX_LOD - 1) - i);
            let h = TERRAIN_DEPTH >> ((MAX_LOD - 1) - i);
            terrain.delta_blocks[i] = vec![0.0; w * h];
        }

        terrain.setup_sky(2500.0, SkyFlags::STARS | SkyFlags::SATELLITES, true);
        terrain.generate_light_source();

        terrain.sky.damage_per_second = 0.0;
        terrain.sky.fog_scalar = 0.85;

        terrain
    }
}

impl Terrain {
    fn init_min_max(&mut self) {
        for i in 0..7 {
            let w = 1 << i;
            let h = 1 << i;

            // Index 1 cuts the whole thing into 4ths, index 2 into 8ths, etc
            self.min_heights[i] = vec![0; w * h];
            self.max_heights[i] = vec![0; w * h];
        }
    }

    fn create_terrain_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();

        for i in 0..TERRAIN_WIDTH * TERRAIN_DEPTH {
            hasher.write_u64::<LittleEndian>(i as u64);
            hasher.write_u8(self.segments[i].y_scalar);
        }

        hasher.finalize()
    }

    /// Builds the min max quadtree data for terrain VSD
    fn build_mix_max(&mut self) {
        debug!("Building min/max table");

        // Calculate our integer y positions (0-255)
        for i in 0..TERRAIN_WIDTH * TERRAIN_DEPTH {
            self.segments[i].y = self.segments[i].y_scalar as f32 * TERRAIN_HEIGHT_INCREMENT;
        }

        let checksum = self.create_terrain_hash();

        match self.checkum {
            None => self.checkum = Some(checksum),
            Some(c) => {
                if c != checksum {
                    self.checkum = Some(checksum);
                }
                else {
                    return;
                }
            }
        }

        #[cfg(not(feature = "dedicated_server"))]
        {
            self.generate_lods();
        }

        for i in 0..7 {
            let row_width = 1 << i;
            let total_rows = 1 << i;
            let mut yspeed_offset = 0;

            for yoffset in 0..total_rows {
                for xoffset in 0..row_width {
                    let mut w = TERRAIN_WIDTH >> i;
                    let mut h = TERRAIN_WIDTH >> i;

                    let mut start = (yoffset * (TERRAIN_WIDTH >> i)) * TERRAIN_WIDTH;
                    start += xoffset * (TERRAIN_WIDTH >> i);

                    let mut min_height = 999i32;
                    let mut max_height = 0i32;

                    if h < TERRAIN_DEPTH {
                        h += 1;
                    }

                    if w < TERRAIN_DEPTH {
                        w += 1;
                    }

                    let mut terrain_offset = 0;

                    for y in 0..h {
                        for x in 0..w {
                            let cell = start + terrain_offset + x;
                            let cell_height = self.segments[cell].y_scalar as i32;

                            if cell_height < min_height {
                                min_height = cell_height as i32;
                            }

                            if cell_height > max_height {
                                max_height = cell_height as i32;
                            }
                        }

                        // XXX: Useless clamping logic, we will never hit a negative value
                        // if min_height < 0 {
                        //     min_height = 0;
                        // }

                        if max_height > 255 {
                            max_height = 255;
                        }

                        self.min_heights[i][yspeed_offset + xoffset] = min_height;
                        self.max_heights[i][yspeed_offset + xoffset] = max_height;

                        terrain_offset += TERRAIN_WIDTH;
                    }
                }

                yspeed_offset += row_width;
            }
        }
    }

    fn generate_lods(&mut self) {
        for i in 0..MAX_LOD - 1 {
            let w = TERRAIN_WIDTH >> ((MAX_LOD - 1) - i);
            let h = TERRAIN_DEPTH >> ((MAX_LOD - 1) - i);

            let simple_mul = 1 << ((MAX_LOD - 1) - i);
            let row_size = TERRAIN_WIDTH / simple_mul;

            for z in 0..h {
                for x in 0..w {
                    let delta = self.recurse_lod_deltas(
                        x * simple_mul,
                        z * simple_mul,
                        (x * simple_mul) + simple_mul,
                        z * simple_mul + simple_mul,
                        i
                    );

                    self.delta_blocks[i][z * row_size + x] = delta;
                }
            }
        }
    }

    fn deform_point(&mut self, x: usize, z: usize, change_height: u8) {
        let mut segment = &mut self.segments[z * TERRAIN_WIDTH + x];

        let change_height = change_height as i32 + segment.y_scalar as i32;
        let change_height = change_height.min(255).max(0) as u8;

        segment.y_scalar = change_height;
        segment.y = change_height as f32 * TERRAIN_HEIGHT_INCREMENT;

        let sx = (x - 1).max(0);
        let sz = (z - 1).max(0);

        // Update min/max
        for i in 0..7 {
            let row_width = 1 << i;
            let div = 256 >> i;

            for t in sz..=z {
                for k in sx..=x {
                    let offset = ((t / div) * row_width) + (k / div);

                    if (segment.y_scalar as i32) > self.max_heights[i][offset] {
                        self.max_heights[i][offset] = segment.y_scalar as i32;
                    }

                    if (segment.y_scalar as i32) < self.min_heights[i][offset] {
                        self.min_heights[i][offset] = segment.y_scalar as i32;
                    }
                }
            }
        }

        // Update normals
        for i in sz..=z {
            for t in sx..=x {
                let seg0 = &self.segments[i * TERRAIN_WIDTH + t];
                let seg1 = &self.segments[(i + 1) * TERRAIN_WIDTH + t];
                let seg2 = &self.segments[((i + 1) * TERRAIN_WIDTH) + t + 1];
                let seg3 = &self.segments[(i * TERRAIN_WIDTH) + t + 1];

                // Do upper left triangle
                let a = Vector {
                    x: t as f32 * TERRAIN_SIZE,
                    y: seg0.y,
                    z: i as f32 * TERRAIN_SIZE
                };

                let b = Vector {
                    x: t as f32 * TERRAIN_SIZE,
                    y: seg1.y,
                    z: (i + 1) as f32 * TERRAIN_SIZE
                };

                let c = Vector {
                    x: (t + 1) as f32 * TERRAIN_SIZE,
                    y: seg2.y,
                    z: (i + 1) as f32 * TERRAIN_SIZE
                };

                Vector::compute_normal_vector(
                    &mut self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + t].upper_left_triangle
                    , &a, &b, &c
                );

                // Now do lower right triangle
                let a = Vector {
                    x: t as f32 * TERRAIN_SIZE,
                    y: seg0.y,
                    z: i as f32 * TERRAIN_SIZE
                };

                let b = Vector {
                    x: (t + 1) as f32 * TERRAIN_SIZE,
                    y: seg2.y,
                    z: (i + 1) as f32 * TERRAIN_SIZE
                };

                let c = Vector {
                    x: (t + 1) as f32 * TERRAIN_SIZE,
                    y: seg3.y,
                    z: i as f32 * TERRAIN_SIZE
                };

                Vector::compute_normal_vector(
                    &mut self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + t].lower_right_triangle
                    , &a, &b, &c
                );
            }
        }
    }

    fn init_normals(&mut self) {
        for i in MAX_LOD - 1..MAX_LOD {
            let w = TERRAIN_WIDTH >> ((MAX_LOD - 1) - i);
            let h = TERRAIN_DEPTH >> ((MAX_LOD - 1) - i);
            self.normals[i] = vec![TerrainNormalPair::default(); w * h];
        }
    }

    fn build_normals(&mut self) {
        let up_normal = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0
        };

        // Set all to be initially up
        for i in 0..TERRAIN_WIDTH * TERRAIN_DEPTH {
            self.normals[MAX_LOD - 1][i].upper_left_triangle = up_normal.clone();
            self.normals[MAX_LOD - 1][i].lower_right_triangle = up_normal.clone();
        }

        for l in MAX_LOD - 1..MAX_LOD {
            let simple_mul = 1 << ((MAX_LOD - 1) - l);

            let mut x = 0;
            let mut z = 0;

            for i in (0..TERRAIN_DEPTH - simple_mul).step_by(simple_mul) {
                for t in (0..TERRAIN_WIDTH - simple_mul).step_by(simple_mul) {
                    let seg0 = &self.segments[i * TERRAIN_WIDTH + t];
                    let seg1 = &self.segments[(i + simple_mul) * TERRAIN_WIDTH + t];
                    let seg2 = &self.segments[((i + simple_mul) * TERRAIN_WIDTH) + t + simple_mul];
                    let seg3 = &self.segments[(i * TERRAIN_WIDTH) + t + simple_mul];

                    // Do upper left triangle
                    let a = Vector {
                        x: t as f32 * TERRAIN_SIZE,
                        y: seg0.y,
                        z: i as f32 * TERRAIN_SIZE
                    };

                    let b = Vector {
                        x: t as f32 * TERRAIN_SIZE,
                        y: seg1.y,
                        z: (i + simple_mul) as f32 * TERRAIN_SIZE
                    };

                    let c = Vector {
                        x: (t + simple_mul) as f32 * TERRAIN_SIZE,
                        y: seg2.y,
                        z: (i + simple_mul) as f32 * TERRAIN_SIZE
                    };

                    Vector::compute_normal_vector(
                        &mut self.normals[l][z * (TERRAIN_WIDTH / simple_mul)].upper_left_triangle
                        , &a, &b, &c
                    );

                    // Now do lower right triangle
                    let a = Vector {
                        x: t as f32 * TERRAIN_SIZE,
                        y: seg0.y,
                        z: i as f32 * TERRAIN_SIZE
                    };

                    let b = Vector {
                        x: (t + simple_mul) as f32 * TERRAIN_SIZE,
                        y: seg2.y,
                        z: (i + simple_mul) as f32 * TERRAIN_SIZE
                    };

                    let c = Vector {
                        x: (t + simple_mul) as f32 * TERRAIN_SIZE,
                        y: seg3.y,
                        z: i as f32 * TERRAIN_SIZE
                    };

                    Vector::compute_normal_vector(
                        &mut self.normals[l][z * (TERRAIN_WIDTH / simple_mul)].lower_right_triangle
                        , &a, &b, &c
                    );

                    x += 1;
                }

                z += 1;
            }
        }
    }

    fn generate_light(&mut self) {
        self.generate_light_source();

        let mut camera_light = self.sky.light_source.clone();
        Vector::normalize(&mut camera_light);

        for i in 0..TERRAIN_WIDTH * TERRAIN_DEPTH {
            let dot =
                (-(camera_light.dot(self.normals[MAX_LOD - 1][i].upper_left_triangle)) + 1.0) / 2.0;
            let l = dot.trunc() as u8;

            self.segments[i].l = l;
            self.segments[i].r = l;
            self.segments[i].g = l;
            self.segments[i].b = l;
        }

        self.update_lightmaps();
    }

    fn update_lightmaps(&mut self) {
        // First make the wraparounds work right

        for i in 0..128 {
            // Lower-left strip
            self.segments[i * TERRAIN_WIDTH].r = self.segments[i * TERRAIN_WIDTH + 128].r;
            self.segments[i * TERRAIN_WIDTH].g = self.segments[i * TERRAIN_WIDTH + 128].g;
            self.segments[i * TERRAIN_WIDTH].b = self.segments[i * TERRAIN_WIDTH + 128].b;

            self.segments[i].r = self.segments[128 * TERRAIN_WIDTH + i].r;
            self.segments[i].g = self.segments[128 * TERRAIN_WIDTH + i].g;
            self.segments[i].b = self.segments[128 * TERRAIN_WIDTH + i].b;

            // Lower-right strip
            self.segments[i * TERRAIN_WIDTH + 255].r = self.segments[i * TERRAIN_WIDTH + 127].r;
            self.segments[i * TERRAIN_WIDTH + 255].g = self.segments[i * TERRAIN_WIDTH + 127].g;
            self.segments[i * TERRAIN_WIDTH + 255].b = self.segments[i * TERRAIN_WIDTH + 127].b;

            self.segments[i + 128].r = self.segments[128 * TERRAIN_WIDTH + i + 128].r;
            self.segments[i + 128].g = self.segments[128 * TERRAIN_WIDTH + i + 128].g;
            self.segments[i + 128].b = self.segments[128 * TERRAIN_WIDTH + i + 128].b;

            // Upper-left strip
            self.segments[(i + 128) * TERRAIN_WIDTH].r =
                self.segments[(i + 128) * TERRAIN_WIDTH + 128].r;
            self.segments[(i + 128) * TERRAIN_WIDTH].g =
                self.segments[(i + 128) * TERRAIN_WIDTH + 128].g;
            self.segments[(i + 128) * TERRAIN_WIDTH].b =
                self.segments[(i + 128) * TERRAIN_WIDTH + 128].b;

            self.segments[255 * TERRAIN_WIDTH + i].r = self.segments[127 * TERRAIN_WIDTH + i].r;
            self.segments[255 * TERRAIN_WIDTH + i].g = self.segments[127 * TERRAIN_WIDTH + i].g;
            self.segments[255 * TERRAIN_WIDTH + i].b = self.segments[127 * TERRAIN_WIDTH + i].b;

            // Upper-right strip
            self.segments[(i + 128) * TERRAIN_WIDTH + 255].r =
                self.segments[(i + 128) * TERRAIN_WIDTH + 127].r;
            self.segments[(i + 128) * TERRAIN_WIDTH + 255].g =
                self.segments[(i + 128) * TERRAIN_WIDTH + 127].g;
            self.segments[(i + 128) * TERRAIN_WIDTH + 255].b =
                self.segments[(i + 128) * TERRAIN_WIDTH + 127].b;

            self.segments[255 * TERRAIN_WIDTH + i + 128].r =
                self.segments[127 * TERRAIN_WIDTH + i + 128].r;
            self.segments[255 * TERRAIN_WIDTH + i + 128].g =
                self.segments[127 * TERRAIN_WIDTH + i + 128].g;
            self.segments[255 * TERRAIN_WIDTH + i + 128].b =
                self.segments[127 * TERRAIN_WIDTH + i + 128].b;
        }

        let lightmap_ref = &self.ligtmaps[0];
        let mut lightmap = lightmap_ref.borrow_mut();
        let w = lightmap.width();

        for i in 0..TERRAIN_DEPTH {
            for t in 0..TERRAIN_WIDTH {
                let seg = &self.segments[i * TERRAIN_WIDTH + t];
                let x = t % 128;
                let y = 127 - (i % 128);
                let which = ((i / 128) * 2) + (t / 128);

                let color = gr_rgb16!(seg.r, seg.g, seg.b);
                let lightmap_ref = &self.ligtmaps[which];
                let mut lightmap = lightmap_ref.borrow_mut();
                
                lightmap.data_mut()[y * w + w] = OPAQUE_FLAG | color;
            }
        }

        for i in 0..4 {
            let lightmap_ref = &self.ligtmaps[i];
            let mut lightmap = lightmap_ref.borrow_mut();
            let flags = lightmap.flags();
            lightmap.set_flags(flags & !LightMapFlags::Limits);
        }
    }

    fn generate_light_source(&mut self) {
        self.sky.light_source.x = self.sky.light_angle.cos();
        self.sky.light_source.z = self.sky.light_angle.sin();
    }

    // TODO: Improve this!
    fn get_highest_delta(deltas: &[f32]) -> Option<usize> {
        let mut high_index = None;
        let mut high_delta = -99999.0;

        for i in 0..deltas.len() {
            if deltas[i] > high_delta {
                high_index = Some(i);
                high_delta = deltas[i];
            }
        }

        high_index
    }

    fn recurse_lod_deltas(&self, x1: usize, y1: usize, x2: usize, y2: usize, lod: usize) -> f32 {
        // assert!(x1 % 2 == 0);
        // assert!(x2 % 2 == 0);
        // assert!(y1 % 2 == 0);
        // assert!(y2 % 2 == 0);

        let midx = ((x2 - x1) / 2) + x1;
        let midy = ((y2 - y1) / 2) + y1;

        let edgex = if x2 == TERRAIN_WIDTH {
            TERRAIN_WIDTH - 1
        } else {
            x2
        };

        let edgey = if y2 == TERRAIN_DEPTH {
            TERRAIN_DEPTH - 1
        } else {
            y2
        };

        // starts from lower left, proceeds clockwise
        let v0 = self.segments[y1 * TERRAIN_WIDTH + x1].y;
        let v1 = self.segments[edgey * TERRAIN_WIDTH + x1].y;
        let v2 = self.segments[edgey * TERRAIN_WIDTH + edgex].y;
        let v3 = self.segments[y1 * TERRAIN_WIDTH + edgex].y;

        let mut deltas = [0.0f32; 6];

        deltas[0] = (self.segments[midy * TERRAIN_WIDTH + midx].y - (((v2 - v0) / 2.0) + v0)).abs();
        deltas[1] = (self.segments[midy * TERRAIN_WIDTH + midx].y - (((v3 - v1) / 2.0) + v1)).abs();

        // left edge
        deltas[2] = (self.segments[midy * TERRAIN_WIDTH + x1].y - (((v1 - v0) / 2.0) + v0)).abs();

        // top edge
        deltas[3] = (self.segments[y2 * TERRAIN_WIDTH + midx].y - (((v2 - v1) / 2.0) + v1)).abs();

        // right edge
        deltas[4] = (self.segments[midy * TERRAIN_WIDTH + x2].y - (((v3 - v2) / 2.0) + v2)).abs();

        // bottom edge
        deltas[5] = (self.segments[y1 * TERRAIN_WIDTH + midx].y - (((v3 - v0) / 2.0) + v0)).abs();

        let mut max_delta = deltas[Self::get_highest_delta(&deltas).unwrap()];

        if lod != MAX_LOD - 2 {
            deltas[0] = self.recurse_lod_deltas(x1, midy, midx, y2, lod + 1);
            deltas[1] = self.recurse_lod_deltas(midx, midy, x2, y2, lod + 1);
            deltas[2] = self.recurse_lod_deltas(midx, y1, x2, midy, lod + 1);
            deltas[3] = self.recurse_lod_deltas(x1, y1, midx, midy, lod + 1);

            if deltas[0] == SHUTOFF_LOD_INVISIBLE
                && deltas[1] == SHUTOFF_LOD_INVISIBLE
                && deltas[2] == SHUTOFF_LOD_INVISIBLE
                && deltas[3] == SHUTOFF_LOD_INVISIBLE
            {
                max_delta = SHUTOFF_LOD_INVISIBLE;
            } else {
                for i in 0..MAX_LOD {
                    if deltas[i] == SHUTOFF_LOD_INVISIBLE {
                        deltas[i] = SHUTOFF_LOD_DELTA;
                    }
                }

                let max_delta_2 = deltas[Self::get_highest_delta(&deltas[0..4]).unwrap()];

                if max_delta_2 > max_delta {
                    max_delta = max_delta_2;
                }
            }
        }

        // Now check if there is anything special about this level of detail that
        // excludes it from being used in the engine
        if lod == MAX_LOD - 2 {
            let mut total_counted = 0;
            let mut total_invisible = 0;

            for i in 0..y2 {
                for t in 0..x2 {
                    if self.segments[i * TERRAIN_WIDTH + t]
                        .flags
                        .contains(TerrainFlags::INVISIBLE)
                    {
                        max_delta = SHUTOFF_LOD_DELTA;
                        total_invisible += 1;
                    }

                    total_counted += 1;
                }
            }

            if total_invisible == total_counted {
                max_delta = SHUTOFF_LOD_INVISIBLE;
            }
        }

        max_delta
    }

    fn get_greatest_slope_change(slopes: &[f32]) -> f32 {
        let mut high_delta = -90000.0;

        for i in 0..slopes.len() {
            for t in 0..slopes.len() {
                if (slopes[t] - slopes[i]).abs() > high_delta {
                    high_delta = (slopes[t] - slopes[i]).abs()
                }
            }
        }

        high_delta
    }

    fn generate_single_lod_delta(&mut self, sx: usize, sz: usize) {
        let chunk_size = 1 << (MAX_LOD - 1);

        let sx = sx * chunk_size as usize;
        let sz = sz * chunk_size as usize;

        let save_x = sx;
        let save_z = sz;

        // Starts from lower-left, going clockwise
        // 0 is lowest_level_detail (blunt)
        for i in 0..MAX_LOD - 1 {
            let w = (chunk_size >> (MAX_LOD - 1) - i);
            let h = (chunk_size >> (MAX_LOD - 1) - i);

            let simple_mul = 1 << ((MAX_LOD - 1) - i);
            let row_size = TERRAIN_WIDTH / simple_mul;

            let sx = save_x / simple_mul;
            let sz = save_z / simple_mul;

            for z in sz..sz + h {
                for x in sx..sx + w {
                    let delta = self.recurse_lod_deltas(
                        x * simple_mul,
                        z * simple_mul,
                        (x * simple_mul) + simple_mul,
                        z * simple_mul + simple_mul,
                        i,
                    );

                    self.delta_blocks[i][z * row_size + x] = delta;
                }
            }
        }
    }

    pub fn setup_sky(&mut self, radius: f32, flags: SkyFlags, randomize: bool) {
        let jump = 65536 / MAX_HORIZON_PIECES;
        let top = ((65536 / 4) * 3) + (65536 / 8);

        self.sky.radius = radius;
        self.sky.flags = flags;

        let horizon_color = (
            gr_color_red!(self.sky.horizon.color),
            gr_color_green!(self.sky.horizon.color),
            gr_color_blue!(self.sky.horizon.color),
        );

        // Figure out where our points in the inside of the sphere are
        for i in 0..6 {
            let increment = 16384 / 5;
            let pitch = Angle(((65536u32 - 16381) as u16).wrapping_add(i as u16 * increment));

            for t in 0..MAX_HORIZON_PIECES {
                let mut vec = &mut self.sky.horizon.vectors[t][i];

                let angle = EulerAngle {
                    pitch: pitch,
                    heading: Angle((t * jump) as u16),
                    bank: Angle(0),
                };

                let mut temp_matrix = Matrix::compute_rotation_3d(&angle);
                *vec = temp_matrix.forward.mul_scalar(radius / 2.0);
            }
        }

        // Now figure out texture UVS
        for i in 0..5 {
            let scalar = i as f32 / 4.0;
            let angle_increment = 65535 / MAX_HORIZON_PIECES;

            for t in 0..MAX_HORIZON_PIECES {
                let mut cur_sin = ((t * angle_increment) as f32).sin() * scalar;
                let mut cur_cos = ((t * angle_increment) as f32).cos() * scalar;

                cur_sin = (cur_sin + 1.0) / 2.0;
                cur_cos = (cur_cos + 1.0) / 2.0;

                self.sky.horizon.u[t][i] = cur_cos;
                self.sky.horizon.v[t][i] = cur_sin;
            }
        }

        let mut highcount = 0; // keep track of what stars are close to the top of the sphere
                               // don't draw too many of them

        if !randomize {
            return;
        }

        extern crate tinyrand;
        use tinyrand::{Rand, StdRand};

        let mut rand = crate::create_rng();

        for i in 0..MAX_STARS {
            let mut star_vec = Vector::default();

            let angle = EulerAngle {
                pitch: Angle::new_random(),
                heading: Angle::new_random(),
                bank: Angle(0),
            };

            let mut temp_matrix = Matrix::compute_rotation_3d(&angle);
            star_vec = temp_matrix.forward.mul_scalar(radius * 500.0);
            self.sky.stars[i].vector = star_vec;

            // Now figure out the color of this star.  The closer to horizon it is, the
            // dimmer it is
            let y_normal = star_vec.y / (radius * 500.0);

            let color_normal = (y_normal * 2.0).min(1.0).max(0.2);

            let color = rand.next_u32() as i32;
            let mut rgb: (i32, i32, i32);

            if color <= 2 {
                rgb = (255, 255, 255);
            } else if color == 3 {
                rgb = (255, 200, 200);
            } else if color == 4 {
                rgb = (255, 200, 255);
            } else {
                rgb = (255, 255, 200);
            }

            rgb = (
                ((1.0 - color_normal).trunc() as i32 * horizon_color.0)
                    + (color_normal.trunc() as i32 * rgb.0),
                ((1.0 - color_normal).trunc() as i32 * horizon_color.1)
                    + (color_normal.trunc() as i32 * rgb.1),
                ((1.0 - color_normal).trunc() as i32 * horizon_color.2)
                    + (color_normal.trunc() as i32 * rgb.2),
            );

            self.sky.stars[i].color = gr_rgb!(rgb.0, rgb.1, rgb.2);
        }

        for i in 0..MAX_SATELLITES {
            let mut satellite_vec = Vector::default();

            let p: u16 = rand.next_u16() % (65336 / 8);
            let top: u32 = ((65536 / 4) * 3) + 4096; // don't do satellites that are straight up

            let angle = EulerAngle {
                pitch: Angle((top as u16 + p) % 65336),
                heading: Angle(((rand.next_u32() * rand.next_u32()) % 65536) as u16),
                bank: Angle(0),
            };

            let mut temp_matrix = Matrix::compute_rotation_3d(&angle);
            let satellite_vec = temp_matrix.forward.mul_scalar(radius * 3.0);
            self.sky.satellites[i].vector = satellite_vec;
            self.sky.satellites[i].size = 500.0;
        }
    }

    pub fn load_height_map(&mut self, bitmap_ref: &SharedMutRef<dyn Bitmap16>) {
        let bitmap = bitmap_ref.as_ref().borrow();
        let width = bitmap.width();
        let height = bitmap.height();
        let data = bitmap.data();

        let terrain_map = vec![0u8; width * height];

        for i in 0..TERRAIN_DEPTH {
            for j in 0..TERRAIN_WIDTH {
                let data_offset = ((i % height) * width) + (j % width);
                let seg_offset = ((TERRAIN_WIDTH - 1) - i) * TERRAIN_WIDTH + j;

                let grayscale_color;

                match bitmap.format() {
                    bitmap::BitmapFormat::Fmt1555 => {
                        grayscale_color = convert_1555_to_grayscale(data[data_offset])
                    }
                    bitmap::BitmapFormat::Fmt4444 => {
                        grayscale_color = convert_4444_to_grayscale(data[data_offset])
                    }
                }

                self.segments[seg_offset].y_scalar = grayscale_color;
            }
        }

        self.build_mix_max();
        self.build_normals();
        self.generate_light();
    }

    pub fn build_normal_for_segment(&mut self, seg: usize) {
        if seg >= (TERRAIN_WIDTH - 1) * (TERRAIN_DEPTH - 1) {
            return;
        }

        let i = seg / TERRAIN_WIDTH;
        let t = seg % TERRAIN_WIDTH;

        let a = Vector {
            x: t as f32 * TERRAIN_SIZE,
            y: self.segments[(i * TERRAIN_WIDTH) + t].y,
            z: i as f32 * TERRAIN_SIZE
        };

        let b = Vector {
            x: t as f32 * TERRAIN_SIZE,
            y: self.segments[(i * TERRAIN_WIDTH) + (t + TERRAIN_WIDTH)].y,
            z: (i + 1) as f32 * TERRAIN_SIZE
        };

        let c = Vector {
            x: (t + 1) as f32 * TERRAIN_SIZE,
            y: self.segments[(i * TERRAIN_WIDTH) + (t + TERRAIN_WIDTH + 1)].y,
            z: (i + 1) as f32 * TERRAIN_SIZE
        };

        Vector::compute_normal_vector(
            &mut self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + t].upper_left_triangle,
            &a, &b, &c
        );

        let b = Vector {
            x: (t + 1) as f32 * TERRAIN_SIZE,
            y: self.segments[(i * TERRAIN_WIDTH) + (t + TERRAIN_WIDTH + 1)].y,
            z: (i + 1) as f32 * TERRAIN_SIZE
        };

        let c = Vector {
            x: t as f32 * TERRAIN_SIZE,
            y: self.segments[(i * TERRAIN_WIDTH) + (t + 1)].y,
            z: i as f32 * TERRAIN_SIZE
        };

        Vector::compute_normal_vector(
            &mut self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + t].lower_right_triangle,
            &a, &b, &c
        );
    }

    pub fn build_lighting_normal_for_segment(&mut self, seg: usize) {
        if seg >= (TERRAIN_WIDTH - 1) * (TERRAIN_DEPTH - 1) {
            return;
        }

        let i = seg / TERRAIN_WIDTH;
        let t = seg % TERRAIN_WIDTH;

        let vback = if i == 0 {
            TERRAIN_DEPTH - 1
        }
        else {
            i - 1
        };

        let hback = if t == 0 {
            TERRAIN_WIDTH - 1
        }
        else {
            t - 1
        };

        let mut temp = Vector::ZERO;

        temp = temp + self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + hback].lower_right_triangle;
        temp = temp + self.normals[MAX_LOD - 1][i * TERRAIN_WIDTH + t].lower_right_triangle;
        temp = temp + self.normals[MAX_LOD - 1][vback * TERRAIN_WIDTH + hback].lower_right_triangle;
        temp = temp + self.normals[MAX_LOD - 1][vback * TERRAIN_WIDTH + t].lower_right_triangle;

        Vector::average(&mut temp, 4);
    }

    pub fn update_single_lightmap(&mut self, lightmap_index: usize) {
        let lightmap_ref = &self.ligtmaps[lightmap_index];
        let mut lightmap = lightmap_ref.borrow_mut();

        let w = lightmap.width();

        lightmap.mark_updated();

        let sx = (lightmap_index % 2) * 128;
        let sz = (lightmap_index / 2) * 128;

        for i in sz..sz + 128 {
            for t in sx..sx + 128 {
                let mut seg = &self.segments[i * TERRAIN_WIDTH + t];

                let color = gr_rgb16!(seg.r, seg.g, seg.b);
                let mut data = lightmap.data_mut();

                let x = t % 128;
                let y = 127 - (i % 128);

                data[y * w + x] = OPAQUE_FLAG | color;
            }
        }
    }

    pub fn lookup_region(&self, num: usize) -> usize {
        let value = self.segments[num].flags.bits() & TerrainFlags::REGION_MASK.bits();
        (value >> 5) as usize
    }

    pub fn clear_node_lists(&mut self) {
        for i in 0..self.node_lists.len() {
            let mut node_list_ref = &self.node_lists[i];
            let mut node_list = node_list_ref.borrow_mut();
            node_list.clear();
        }
    }
}