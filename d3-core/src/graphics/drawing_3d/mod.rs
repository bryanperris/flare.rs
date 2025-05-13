#[cfg(test)]
mod tests;

pub mod conversions;
pub mod legacy_soft;

use crate::{
    common::SharedMutRef,
    math::{
        DotProduct,
        matrix::{Matrix, Matrix4},
        vector::{Vector, Vector4},
    },
};
use bitflags::bitflags;

use anyhow::Result;

use super::{bitmap::Bitmap16, ddgr_color, rendering::Renderer, MapSourceType16};

pub enum Point3Kind {
    Original(Point3),
    Temporary(Point3),
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct PointFlags: u8 {
        const NONE        = 0b0000_0000;
        /// The point has been projected, so `sx, sy` are valid.
        const PROJECTED   = 0b0000_0001;
        /// The point is beyond the fog zone.
        const FAR_ALPHA   = 0b0000_0010;
        /// The point was created during clipping.
        const CLIPPER_TEMP_POINT  = 0b0000_0100;
        /// The point has UV texture coordinates set.
        const UV          = 0b0000_1000;
        /// The point has lighting values set.
        const LIGHTING    = 0b0001_0000;
        /// The point has RGBA lighting values set.
        const RGBA        = 0b0010_0000;
        /// The point has secondary UV coordinates (e.g., lightmap UVs).
        const UV2         = 0b0100_0000;
        /// The point represents an original (non-transformed) point.
        const ORIGINAL_POINT   = 0b1000_0000;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ClippingCode: u8 {
        const OFF_LEFT   = 0b0000_0001; // 1
        const OFF_RIGHT  = 0b0000_0010; // 2
        const OFF_BOT    = 0b0000_0100; // 4
        const OFF_TOP    = 0b0000_1000; // 8
        const OFF_FAR    = 0b0001_0000; // 16
        const OFF_CUSTOM = 0b0010_0000; // 32
        const BEHIND     = 0b1000_0000; // 128
    }
}

// X, Y should represent ast top-left corner of the screen
pub struct ScreenViewPort {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub aspect: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct UVL {
    pub u: f32,
    pub v: f32,
    pub u2: f32,
    pub v2: f32,
    pub light_intensity: f32,
    pub light_r: f32,
    pub light_g: f32,
    pub light_b: f32,
    pub light_a: f32,
}

impl Default for UVL {
    fn default() -> Self {
        Self {
            u: Default::default(),
            v: Default::default(),
            u2: Default::default(),
            v2: Default::default(),
            light_intensity: Default::default(),
            light_r: Default::default(),
            light_g: Default::default(),
            light_b: Default::default(),
            light_a: Default::default(),
        }
    }
}

mod math {
    use crate::math::{
        DotProduct,
        matrix::{Matrix, Matrix4},
        vector::Vector,
    };

    /// Compute the vector through that given point
    pub fn point_to_vec(
        v: &Vector,
        point_2d: (usize, usize),
        winres_2: (f32, f32),
        scale: &Vector,
        m: &Matrix,
    ) -> Vector {
        let mut v = Vector {
            x: (((point_2d.0 as f32 - winres_2.0) / winres_2.0) * scale.z / scale.x),
            y: (((point_2d.1 as f32 - winres_2.1) / winres_2.1) * scale.z / scale.y),
            z: 1.0,
        };

        let _ = Vector::normalize(&mut v);

        let m = m.transpose();

        v * m
    }
}

/// Used to store rotated points for mines.  Has frame count to indicate
/// if rotated, and flag to indicate if projected.

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone)]
pub struct Point3 {
    pub screen_x: f32,
    pub screen_y: f32,
    pub clipping_codes: ClippingCode,
    pub flags: PointFlags,
    pub transform: Vector, // the origin transformed
    pub origin: Vector,
    pub uvl: UVL,
}

impl Point3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            screen_x: 0.0,
            screen_y: 0.0,
            clipping_codes: ClippingCode::empty(),
            flags: PointFlags::empty(),
            transform: Vector { x: x, y: y, z: z },
            origin: Vector::ZERO,
            uvl: UVL {
                u: 0.0,
                v: 0.0,
                u2: 0.0,
                v2: 0.0,
                light_intensity: 0.0,
                light_r: 0.0,
                light_g: 0.0,
                light_b: 0.0,
                light_a: 0.0,
            },
        }
    }

    pub fn x(&self) -> f32 {
        self.transform.x
    }

    pub fn set_x(&mut self, x: f32) {
        self.transform.x = x;
    }

    pub fn y(&self) -> f32 {
        self.transform.y
    }

    pub fn set_y(&mut self, y: f32) {
        self.transform.y = y;
    }

    pub fn z(&self) -> f32 {
        self.transform.z
    }

    pub fn set_z(&mut self, z: f32) {
        self.transform.z = z;
    }

    pub fn u(&self) -> f32 {
        self.uvl.u
    }

    pub fn set_u(&mut self, u: f32) {
        self.uvl.u = u;
    }

    pub fn v(&self) -> f32 {
        self.uvl.v
    }

    pub fn set_v(&mut self, v: f32) {
        self.uvl.v = v;
    }

    pub fn u2(&self) -> f32 {
        self.uvl.u2
    }

    pub fn set_u2(&mut self, u: f32) {
        self.uvl.u2 = u;
    }

    pub fn v2(&self) -> f32 {
        self.uvl.v2
    }

    pub fn set_v2(&mut self, v: f32) {
        self.uvl.v2 = v;
    }

    pub fn light(&self) -> f32 {
        self.uvl.light_intensity
    }

    pub fn set_light(&mut self, light: f32) {
        self.uvl.light_intensity = light;
    }

    /// Transforms a world-space point into view (camera) space, applying translation and rotation.
    ///
    /// # Arguments
    ///
    /// * `point` - A reference to the 3D point in world space.
    /// * `view` - A tuple containing:
    ///     - The view (camera) position as a `&Vector`
    ///     - The view orientation as a `&Matrix`
    /// * `clip` - A tuple containing:
    ///     - The Z clip value as `f32`
    ///     - An optional custom clip definition (`&Option<CustomClip>`)
    ///
    /// # Behavior
    ///
    /// - Translates the point relative to the camera's position.
    /// - Rotates the point using the camera's view matrix.
    /// - Marks the point as an original (non-interpolated) vertex.
    /// - Computes and sets the clip code for visibility testing.
    ///
    /// # Example
    /// ```
    /// let point = Vector::new(1.0, 2.0, 3.0);
    /// let view_position = Vector::zero();
    /// let view_matrix = Matrix::identity();
    /// let mut p = MyPoint::default();
    /// p.apply_view_transform(&point, (&view_position, &view_matrix), (1.0, &None));
    /// ```
    pub fn apply_view_transform(
        &mut self,
        point: &Vector,
        view: &Camera,
        clip: (f32, &Option<CustomClip>),
    ) {
        self.origin = point.clone();

        // Compute the point offset from the view position
        let v = *point - view.position;

        // Compute the point rotation by the view orientation
        self.transform = v * view.orientation;

        self.flags.insert(PointFlags::ORIGINAL_POINT);
        self.compute_clipcode(clip.0, clip.1);
    }

    pub fn apply_projection(&mut self, winres_2: (f32, f32)) {
        if self.flags.contains(PointFlags::PROJECTED)
            || self.clipping_codes.contains(ClippingCode::BEHIND)
        {
            return;
        }

        let one_over_z = 1.0 / self.z();
        self.screen_x = winres_2.0 + (self.x() * (winres_2.0 * one_over_z));
        self.screen_y = winres_2.1 + (self.y() * (winres_2.1 * one_over_z));
        self.flags.insert(PointFlags::PROJECTED);
    }

    pub fn add_delta(&mut self, p: Self, delta: &Vector, clip: (f32, &Option<CustomClip>)) {
        self.transform = p.transform + *delta;
        self.flags = PointFlags::empty();
        self.compute_clipcode(clip.0, clip.1);
    }

    pub fn compute_clipcode(&mut self, clip_far_z: f32, custom_clip: &Option<CustomClip>) {
        self.clipping_codes = ClippingCode::empty();

        if self.x() > self.z() {
            self.clipping_codes.insert(ClippingCode::OFF_RIGHT);
        }

        if self.y() > self.z() {
            self.clipping_codes.insert(ClippingCode::OFF_TOP);
        }

        if self.x() < -self.z() {
            self.clipping_codes.insert(ClippingCode::OFF_LEFT);
        }

        if self.y() < -self.z() {
            self.clipping_codes.insert(ClippingCode::OFF_BOT);
        }

        if self.z() < 0.0 {
            self.clipping_codes.insert(ClippingCode::BEHIND);
        }

        if self.z() > clip_far_z {
            self.clipping_codes.insert(ClippingCode::OFF_FAR);
        }

        match custom_clip {
            Some(cc) => {
                let mut vec = self.transform - cc.clipping_plane_point;
                vec.x /= cc.matrix_scale.x;
                vec.y /= cc.matrix_scale.y;
                vec.z /= cc.matrix_scale.z;

                let mut dp = vec * cc.clipping_plane;

                if (dp < -0.005) {
                    self.clipping_codes.insert(ClippingCode::OFF_CUSTOM);
                }
            }
            None => {}
        }
    }
}

impl Default for Point3 {
    fn default() -> Self {
        Self {
            screen_x: Default::default(),
            screen_y: Default::default(),
            clipping_codes: ClippingCode::empty(),
            flags: PointFlags::NONE,
            transform: Default::default(),
            origin: Default::default(),
            uvl: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vector,
    pub scale: Vector,
    pub transformation: Matrix, // with scale
    pub orientation: Matrix,    // without scale
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vector::ZERO,
            scale: Vector {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            transformation: Matrix::IDENTITY,
            orientation: Matrix::IDENTITY,
            zoom: 1.0,
        }
    }
}

pub trait RenderSetupState {
    fn set_aspect_ratio(&mut self, value: f32);
    fn get_aspect_ratio(&self) -> f32;
    fn set_clipping_far_z(&mut self, value: f32);

    fn reset_clipping_far_z(&mut self) {
        self.set_clipping_far_z(f32::MAX);
    }

    // fn compute_viewport_matrix(&self, viewport: &ScreenRect) -> Matrix4;
    // fn compute_projection_matrix(&self, viewport: &ScreenRect, zoom: f32) -> Matrix4;
    // fn compute_viewmodel_matrix(&self, view: &Camera) -> Matrix4;
    // fn compute_point_transform(&self, point: &Vector4, m: &Matrix4) -> Vector4;
    // fn compute_matrix_product(&self, a: &Matrix4, b: &Matrix4) -> Matrix4;
    fn update_transforms(&mut self);
    fn on_frame_start(&mut self, viewport: &ScreenViewPort, view: &Camera);
}

#[derive(Debug, Copy, Clone)]
pub struct CustomClip {
    pub clipping_plane_point: Vector,
    pub clipping_plane: Vector,
    pub matrix_scale: Vector,
}

pub trait RenderPipeline<R: Renderer> {
    fn draw_line(&self, renderer: &mut R, color: ddgr_color, p0: &Point3, p1: Point3)
    -> Result<()>;

    /// draws a line based on the current setting of render states. takes two points.  returns true if drew
    fn draw_special_line(
        &self,
        renderer: &mut R,
        color: ddgr_color,
        p0: &Point3,
        p1: Point3,
    ) -> Result<()>;

    //g3_CheckNormalFacing
    // DoFacingCheck

    fn draw_poly<B: Bitmap16>(
        &self,
        renderer: &mut R,
        pointlist: &[Point3],
        map_source: Option<MapSourceType16>,
    ) -> Result<Option<usize>>;
}
