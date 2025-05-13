use core::{ops::Neg, pin, str::Lines};
use std::rc::Rc;

use bitflags::Flags;
use tracing::instrument;

use crate::math::{
    DotProduct,
    matrix::{Matrix, Matrix4},
    vector::{Vector, Vector4},
};

use super::{Camera, ClippingCode, CustomClip, Point3, PointFlags, RenderSetupState, ScreenViewPort};

#[derive(Debug, Clone)]
pub struct Transformation {
    pub last_view: Camera,
    pub view: Camera,
    pub transformation: Matrix4,
}

impl Transformation {
    pub fn new(position: &Vector, orientation: &Matrix, camera: &Camera) -> Self {
        let last_camera = camera.to_owned();

        // subtract object position from view position
        let v = last_camera.position - *position;

        // rotate object matrix through view_matrix (vm = ob * vm)
        let m = orientation.transpose();

        let mut next_camera = Camera {
            position: camera.position - *position,
            scale: camera.scale.to_owned(),
            transformation: m * camera.transformation,
            orientation: m * camera.orientation,
            zoom: camera.zoom,
        };

        let t =
            math::compute_viewmodel_matrix(&next_camera.position, &next_camera.orientation);

        Self {
            last_view: last_camera,
            view: next_camera,
            transformation: t,
        }
    }

    pub fn compute_transformation(&self) -> Matrix4 {
        let wm: Matrix4 = self.view.transformation.into();
        wm * self.transformation
    }
}

/// Represents a complete transformation pipeline from model space
/// to screen space in a 3D graphics system.
#[derive(Debug, Clone)]
pub struct TransformPipeline {
    /// A stack of model and local-space transformations applied before view.
    ///
    /// This typically includes object-space transforms such as scaling,
    /// rotation, and translation relative to the world.
    pub modelview_stack: Vec<Transformation>,

    /// The active camera in the scene.
    ///
    /// The camera provides the view transformation from world space to
    /// eye/view space, usually through a view matrix computed from its position
    /// and orientation.
    pub view: Camera,

    /// The projection matrix used to transform view space to clip space.
    ///
    /// This may be a perspective or orthographic projection depending on the
    /// rendering setup.
    pub projection: Matrix4,

    /// The viewport transformation matrix from normalized device coordinates (NDC)
    /// to screen or framebuffer coordinates.
    ///
    /// This typically includes scaling and translation to convert NDC [-1, 1] into
    /// screen-space [0, width] × [0, height].
    pub viewport: Matrix4,
}

impl Default for TransformPipeline {
    fn default() -> Self {
        Self {
            modelview_stack: Vec::new(),
            view: Default::default(),
            projection: Matrix4::identity(),
            viewport: Matrix4::identity(),
        }
    }
}

impl TransformPipeline {
    pub fn compute_final_transform(&mut self) -> Matrix4 {
        let m = match self.modelview_stack.last() {
            Some(mv) => mv.compute_transformation() * self.projection,
            None => {
                let v: Matrix4 = self.view.transformation.into();
                v * self.projection
            }
        };

        m * self.viewport
    }
}

mod math {
    use crate::{graphics::drawing_3d::ScreenViewPort, math::{matrix::{Matrix, Matrix4}, vector::Vector, DotProduct}};

    pub fn compute_viewport_matrix(viewport: &ScreenViewPort) -> Matrix4 {
        let w2 = viewport.width as f32 * 0.5;
        let h2 = viewport.height as f32 * 0.5;
        let x = w2 + viewport.x as f32;
        let y = h2 + viewport.y as f32;

        Matrix4::new(
            w2, 0.0, 0.0, 0.0, 0.0, h2, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, 0.0, 1.0,
        )
    }

    pub fn compute_projection_matrix(viewport: &ScreenViewPort, zoom: f32) -> Matrix4 {
        let s = viewport.aspect * viewport.height as f32 / viewport.width as f32;

        // calc 1/tan(fov) which is the focal length
        let f = 1.0 / zoom;
        let fs = f * s;

        Matrix4::new(
            f, 0.0, 0.0, 0.0, 0.0, fs, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0, 0.0,
        )
    }

    pub fn compute_viewmodel_matrix(view_position: &Vector, view_orientation: &Matrix) -> Matrix4 {
        let local_orientation = view_orientation;
        let local_position = -(*view_position);

        Matrix4::new(
            local_orientation.right.x,
            local_orientation.up.x,
            local_orientation.forward.x,
            0.0,
            local_orientation.right.y,
            local_orientation.up.y,
            local_orientation.forward.y,
            0.0,
            local_orientation.right.z,
            local_orientation.up.z,
            local_orientation.forward.z,
            0.0,
            local_position.dot(local_orientation.right),
            local_position.dot(local_orientation.up),
            local_position.dot(local_orientation.forward),
            1.0,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SoftRenderSetup {
    pub aspect_override: Option<f32>, // user override stored as w/h
    pub aspect: f32,
    pub window_width: usize,
    pub window_height: usize,

    /// width / 2
    pub window_width_2: f32,
    /// height / 2
    pub window_height_2: f32,

    pub xform_pipeline: TransformPipeline,
    pub xform: Matrix4,

    pub clipper_plane_point: Vector,
    pub clipper_far_z: f32,
    pub clipper_custom: Option<CustomClip>,
}

#[derive(Debug, Copy, Clone)]
enum ClipperPoint3Index {
    Original(usize),
    Temporary(usize),
}

impl From<ClipperPoint3Index> for usize {
    fn from(value: ClipperPoint3Index) -> Self {
        match value {
            ClipperPoint3Index::Original(v) => v,
            ClipperPoint3Index::Temporary(v) => v,
        }
    }
}

impl From<&ClipperPoint3Index> for usize {
    fn from(value: &ClipperPoint3Index) -> Self {
        match value {
            ClipperPoint3Index::Original(v) => *v,
            ClipperPoint3Index::Temporary(v) => *v,
        }
    }
}

#[derive(Debug, Clone)]
struct ClipperPointList {
    pointlist: Box<[Point3]>,
    freelist: Vec<ClipperPoint3Index>,
}

impl ClipperPointList {
    fn init_freepoints(&mut self) {
        self.freelist.clear();
        for i in 0..self.pointlist.len() {
            self.freelist.push(ClipperPoint3Index::Original(i));
        }
    }

    fn get_temp_point(&mut self) -> ClipperPoint3Index {
        let p = self.freelist.pop().unwrap();
        ClipperPoint3Index::Temporary(p.into())
    }

    fn free_temp_point(&mut self, point_index: ClipperPoint3Index) {
        self.freelist.push(point_index);
    }

    fn get_point_mut_ref(&mut self, index: usize) -> &mut Point3 {
        &mut self.pointlist[index]
    }

    fn get_point_ref(&self, index: usize) -> &Point3 {
        &self.pointlist[index]
    }
}

impl Vector {
    pub fn rotate_delta_x(&mut self, dx: f32, view: &Matrix) {
        self.x = view.right.x * dx;
        self.y = view.up.x * dx;
        self.z = view.forward.x * dx;
    }

    pub fn rotate_delta_y(&mut self, dy: f32, view: &Matrix) {
        self.x = view.right.y * dy;
        self.y = view.up.y * dy;
        self.z = view.forward.y * dy;
    }

    pub fn rotate_delta_z(&mut self, dz: f32, view: &Matrix) {
        self.x = view.right.z * dz;
        self.y = view.up.z * dz;
        self.z = view.forward.z * dz;
    }

    pub fn rotate_delta(&mut self, v: &Vector, view: &Matrix) {
        *self = *v * view;
    }

    // calculate the depth of a point - returns the z coord of the rotated point
    pub fn compute_z(&self, view: (&Vector, &Matrix)) -> f32 {
        let z = (self.x - view.0.x);
        let z = z + ((self.y - view.0.y) * view.1.forward.y);
        let z = z + ((self.z - view.0.z) * view.1.forward.z);
        z
    }
}

impl SoftRenderSetup {
    fn _get_aspect_ratio(&self) -> f32 {
        match self.aspect_override {
            Some(a) => self.aspect * 4.0 / 3.0 / a,
            None => self.aspect,
        }
    }

    fn compute_point_attributes(
        off_point: &Point3,
        on_point: &Point3,
        dest_point: &mut Point3,
        k: f32,
    ) {
        dest_point.set_z(on_point.z() + ((off_point.z() - on_point.z()) * k));
        dest_point.set_x(on_point.x() + ((off_point.x() - on_point.x()) * k));
        dest_point.set_y(on_point.y() + ((off_point.z() - on_point.y()) * k));

        if on_point.flags.contains(PointFlags::UV) {
            dest_point.set_u(on_point.u() + ((off_point.u() - on_point.u()) * k));
            dest_point.set_v(on_point.v() + ((off_point.v() - on_point.v2()) * k));
            dest_point.flags.insert(PointFlags::UV);
        }

        if on_point.flags.contains(PointFlags::UV2) {
            dest_point.set_u2(on_point.u2() + ((off_point.u2() - on_point.u2()) * k));
            dest_point.set_v2(on_point.v2() + ((off_point.v2() - on_point.v2()) * k));
            dest_point.flags.insert(PointFlags::UV2);
        }

        if on_point.flags.contains(PointFlags::LIGHTING) {
            dest_point.set_light(on_point.light() + ((off_point.light() - on_point.light()) * k));
            dest_point.flags.insert(PointFlags::LIGHTING);
        }

        if on_point.flags.contains(PointFlags::RGBA) {
            dest_point.uvl.light_r =
                on_point.uvl.light_r + ((off_point.uvl.light_r - on_point.uvl.light_r) * k);
            dest_point.uvl.light_g =
                on_point.uvl.light_g + ((off_point.uvl.light_g - on_point.uvl.light_g) * k);
            dest_point.uvl.light_b =
                on_point.uvl.light_b + ((off_point.uvl.light_b - on_point.uvl.light_b) * k);
            dest_point.uvl.light_a =
                on_point.uvl.light_a + ((off_point.uvl.light_a - on_point.uvl.light_a) * k);
            dest_point.flags.insert(PointFlags::RGBA);
        }
    }

    // Clips a polygon
    // Parameters:	pointlist - pointer to a list of pointers to points
    //					nv - the number of points in the polygon
    //					cc - the clip codes for this polygon
    // Returns:	a pointer to a list of pointer of points in the clipped polygon
    // NOTE: You MUST call g3_FreeTempPoints() when you're done with the clipped polygon
    pub fn clipper_clip_polygon(
        &mut self,
        mut pointlist: Vec<Point3>,
        cc_or: &mut ClippingCode,
        cc_and: &mut ClippingCode,
    ) -> Vec<Point3> {
        for flag in ClippingCode::iter(&ClippingCode::all()) {
            if cc_or.contains(flag) {
                let mut clipper_pointlist = ClipperPointList {
                    pointlist: pointlist.into_boxed_slice(),
                    freelist: Vec::new(),
                };

                clipper_pointlist.init_freepoints();

                pointlist = self.clipper_clip_plane(clipper_pointlist, flag, cc_or, cc_and);

                if !cc_and.is_empty() {
                    return pointlist;
                }
            }
        }

        pointlist
    }

    #[tracing::instrument]
    fn clipper_clip_plane(
        &mut self,
        mut clipping_pointlist: ClipperPointList,
        clip_code: ClippingCode,
        cc_or: &mut ClippingCode,
        cc_and: &mut ClippingCode,
    ) -> Vec<Point3> {
        // Init codes
        *cc_and = ClippingCode::all();
        *cc_or = ClippingCode::empty();

        let mut new_pointlist: Vec<usize> = Vec::new();

        let mut prev = clipping_pointlist.pointlist.len() - 1;
        let mut next = 1;

        for i in 0..clipping_pointlist.pointlist.len() {
            let mut cur = ClipperPoint3Index::Original(i);
            let mut off = ClipperPoint3Index::Original(i);
            let mut temp_1: Option<ClipperPoint3Index> = None;
            let mut temp_2: Option<ClipperPoint3Index> = None;

            if clipping_pointlist.pointlist[i]
                .clipping_codes
                .contains(clip_code)
            {
                trace!("Found vertex point with clip code");

                if !clipping_pointlist
                    .get_point_ref(prev)
                    .clipping_codes
                    .contains(clip_code)
                {
                    let mut on = ClipperPoint3Index::Original(prev);

                    trace!("prev point does not have {:?} set", clip_code);

                    temp_1 =
                        Some(self.clipper_clip_edge(clip_code, &mut clipping_pointlist, &on, &off));
                    new_pointlist.push(temp_1.unwrap().into());
                }

                if !clipping_pointlist
                    .get_point_ref(next)
                    .clipping_codes
                    .contains(clip_code)
                {
                    let mut on = ClipperPoint3Index::Original(next);

                    trace!("next point does not have {:?} set", clip_code);

                    temp_2 =
                        Some(self.clipper_clip_edge(clip_code, &mut clipping_pointlist, &on, &off));
                    new_pointlist.push(temp_2.unwrap().into());
                }

                if let Some(v) = temp_1 {
                    if usize::from(v) == usize::from(cur) {
                        match v {
                            ClipperPoint3Index::Temporary(_) => {
                                clipping_pointlist.free_temp_point(v);
                            }
                            _ => {}
                        }
                    }
                }

                if let Some(v) = temp_2 {
                    if usize::from(v) == usize::from(cur) {
                        match v {
                            ClipperPoint3Index::Temporary(_) => {
                                clipping_pointlist.free_temp_point(v);
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                *cc_or |= clipping_pointlist.pointlist[i].clipping_codes;
                *cc_and &= clipping_pointlist.pointlist[i].clipping_codes;
                new_pointlist.push(i);
            }

            prev = i;

            if (next + 1) >= clipping_pointlist.pointlist.len() {
                next = 0;
            } else {
                next += 1;
            }
        }

        let mut original_pointlist: Vec<Option<Point3>> = clipping_pointlist
            .pointlist
            .into_iter()
            .filter_map(|p| Some(Some(p)))
            .collect();

        new_pointlist
            .drain(..)
            .filter_map(|pi| original_pointlist[pi].take())
            .collect()
    }

    //// clips an edge against one plane.
    fn clipper_clip_edge(
        &mut self,
        clip_code: ClippingCode,
        pointlist: &mut ClipperPointList,
        on_point: &ClipperPoint3Index,
        off_point: &ClipperPoint3Index,
    ) -> ClipperPoint3Index {
        // compute clipping value k = (xs-zs) / (xs-xe-zs+ze)
        // use x or y as appropriate, and negate x/y value as appropriate
        let pointlist_ptr = pointlist.pointlist.as_mut_ptr();
        let on_point_index: usize = on_point.into();
        let off_point_index: usize = on_point.into();

        assert!(on_point_index < pointlist.pointlist.len());
        assert!(off_point_index < pointlist.pointlist.len());

        let (on, off): (&mut Point3, &mut Point3) = unsafe {
            (
                &mut *pointlist_ptr.add(on_point_index),
                &mut *pointlist_ptr.add(off_point_index),
            )
        };

        if clip_code.contains(ClippingCode::OFF_FAR) {
            return self.clipper_clip_far_edge(pointlist, on_point, off_point);
        }

        if clip_code.contains(ClippingCode::OFF_CUSTOM) && self.clipper_custom.is_some() {
            return self.clipper_clip_custom_edge(pointlist, on_point, off_point);
        }

        let (mut a, mut b) = if clip_code.contains(ClippingCode::OFF_RIGHT | ClippingCode::OFF_LEFT)
        {
            (on.x(), off.x())
        } else {
            (on.y(), off.y())
        };

        if clip_code.contains(ClippingCode::OFF_LEFT) || clip_code.contains(ClippingCode::OFF_BOT) {
            a = -a;
            b = -b;
        }

        // //(xs-zs) / (xs-zs-xe+ze)
        let v = a - on.z();
        let k = v / (v - b + off.z());

        let mut point = pointlist.get_temp_point();
        let p = pointlist.get_point_mut_ref(point.into());
        Self::compute_point_attributes(&off, &on, p, k);
        p.compute_clipcode(self.clipper_far_z, &self.clipper_custom);
        point
    }

    fn clipper_clip_far_edge(
        &mut self,
        pointlist: &mut ClipperPointList,
        on_point: &ClipperPoint3Index,
        off_point: &ClipperPoint3Index,
    ) -> ClipperPoint3Index {
        let pointlist_ptr = pointlist.pointlist.as_mut_ptr();
        let on_point_index: usize = on_point.into();
        let off_point_index: usize = off_point.into();

        assert!(on_point_index < pointlist.pointlist.len());
        assert!(off_point_index < pointlist.pointlist.len());

        let (on, off): (&mut Point3, &mut Point3) = unsafe {
            (
                &mut *pointlist_ptr.add(on_point_index),
                &mut *pointlist_ptr.add(off_point_index),
            )
        };

        let z_on = on.transform.x;
        let z_off = (*off).transform.z;
        let k = 1.0 - ((z_off - self.clipper_far_z) / (z_off - z_on));

        let mut point = pointlist.get_temp_point();
        let p = pointlist.get_point_mut_ref(point.into());
        Self::compute_point_attributes(&off, &on, p, k);
        p.compute_clipcode(self.clipper_far_z, &self.clipper_custom);
        point
    }

    // Clips an edge against the far plane
    fn clipper_clip_custom_edge(
        &mut self,
        pointlist: &mut ClipperPointList,
        on_point: &ClipperPoint3Index,
        off_point: &ClipperPoint3Index,
    ) -> ClipperPoint3Index {
        let pointlist_ptr = pointlist.pointlist.as_mut_ptr();
        let on_point_index: usize = on_point.into();
        let off_point_index: usize = off_point.into();

        assert!(on_point_index < pointlist.pointlist.len());
        assert!(off_point_index < pointlist.pointlist.len());

        let (on, off): (&mut Point3, &mut Point3) = unsafe {
            (
                &mut *pointlist_ptr.add(on_point_index),
                &mut *pointlist_ptr.add(off_point_index),
            )
        };

        let mut ray_direction = off.transform - on.transform;
        ray_direction.x /= self.xform_pipeline.view.scale.x;
        ray_direction.y /= self.xform_pipeline.view.scale.y;
        ray_direction.z /= self.xform_pipeline.view.scale.z;

        let den = -(self.clipper_plane_point * ray_direction);

        let k = if den == 0.0 {
            1.0
        } else {
            let mut w = on.transform - self.clipper_plane_point;
            w.x /= self.xform_pipeline.view.scale.x;
            w.y /= self.xform_pipeline.view.scale.y;
            w.z /= self.xform_pipeline.view.scale.z;

            (self.clipper_plane_point * w) / den
        };

        let mut point = pointlist.get_temp_point();
        let p = pointlist.get_point_mut_ref(point.into());
        Self::compute_point_attributes(&off, &on, p, k);
        p.compute_clipcode(self.clipper_far_z, &self.clipper_custom);
        point
    }

    //// clips a line to the viewing pyramid.
    //// TODO: p0 and p1 need to be mutable slices
    //// This function needs to be re-worked
    // fn clipper_clip_line(&mut self, p0: &mut , p1: Point3, codes_or: ClippingCode) {
    //     let mut codes_or = codes_or;

    //     let mut p0 = p0;
    //     let mut p1 = p1;

    //     for flag in ClippingCode::iter(&ClippingCode::all()) {
    //         if codes_or.contains(flag) {
    //             if p0.clipping_codes.contains(flag) {
    //                 let mut temp = p0;
    //                 p0 = p1;
    //                 p1 = p0;
    //             }

    //             let mut old_point = p1;

    //             p1 = self.clipper_clip_edge(flag, p0, p1.to_owned());

    //             codes_or = p0.clipping_codes | p1.clipping_codes;

    //             if old_point.flags.contains(PointFlags::)
    //         }
    //     }
    // }
    /*
    void ClipLine(g3Point **p0, g3Point **p1, ubyte codes_or) {
      int plane_flag;
      g3Point *old_p1;

      // might have these left over
      //(*p0)->p3_flags &= ~(PF_UV|PF_L|PF_RGBA|PF_UV2);
      //(*p1)->p3_flags &= ~(PF_UV|PF_L|PF_RGBA|PF_UV2);

      for (plane_flag = 1; plane_flag <= 32; plane_flag <<= 1) {
        if (codes_or & plane_flag) {
          if ((*p0)->p3_codes & plane_flag) {
            // swap!
            g3Point *t = *p0;
            *p0 = *p1;
            *p1 = t;
          }

          old_p1 = *p1;

          *p1 = ClipEdge(plane_flag, *p0, *p1);

          codes_or = (*p0)->p3_codes | (*p1)->p3_codes; // get new codes

          if (old_p1->p3_flags & PF_TEMP_POINT) {
            FreeTempPoint(old_p1);
          }
        }
      }
    }
     */
}

impl RenderSetupState for SoftRenderSetup {
    fn set_aspect_ratio(&mut self, value: f32) {
        self.aspect = value;
    }

    fn get_aspect_ratio(&self) -> f32 {
        self.aspect
    }

    fn set_clipping_far_z(&mut self, value: f32) {
        self.clipper_far_z = value;
    }

    // #[inline]
    // fn compute_point_transform(&self, point: &Vector4, m: &Matrix4) -> Vector4 {
    //     *m * *point
    // }

    // #[inline]
    // fn compute_matrix_product(&self, a: &Matrix4, b: &Matrix4) -> Matrix4 {
    //     *a * *b
    // }

    #[inline]
    fn update_transforms(&mut self) {
        self.xform_pipeline.compute_final_transform();
    }

    fn on_frame_start(&mut self, viewport: &ScreenViewPort, view: &Camera) {
        // self.xform_pipeline.viewport = math::compute_viewport_matrix(viewport);
        // self.xform_pipeline.projection = math::compute_projection_matrix(viewport, view.zoom);
        let mv = math::compute_viewmodel_matrix(&self.xform_pipeline.view.position, &self.xform_pipeline.view.orientation);

        self.xform_pipeline.modelview_stack.push(Transformation {
            last_view: view.to_owned(),
            view: view.to_owned(),
            transformation: mv,
        });

        self.xform = self.xform_pipeline.compute_final_transform();

        // Setup projection
        let window_w2 = viewport.width as f32 * 0.5;
        let window_h2 = viewport.height as f32 * 0.5;

        // Compute aspect ratio for this window
        let aspect = self._get_aspect_ratio();
        let s = aspect * viewport.height as f32 / viewport.width as f32;

        // XXX: Should this have been 1.0f?
        if s <= 0.0 {
            // Scale X
            self.xform_pipeline.view.scale.x = s;
            self.xform_pipeline.view.scale.y = 1.0;
        } else {
            self.xform_pipeline.view.scale.x = 1.0;
            self.xform_pipeline.view.scale.y = 1.0 / s;
        }

        self.xform_pipeline.view.scale.z = 1.0;

        // setup view
        self.xform_pipeline.view.position = view.position.to_owned();
        self.xform_pipeline.view.zoom = view.zoom;
        self.xform_pipeline.view.orientation = view.orientation.to_owned();

        let mut scale = Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };

        // compyute matrix scale for zoom and aspect ratio
        if view.zoom <= 1.0 {
            // Zoom in by scaling z
            scale.z *= view.zoom;
        } else {
            // Zoom out by scaling x and y
            let focal_len = 1.0 / view.zoom;
            scale.x *= focal_len;
            scale.y *= focal_len;
        }

        // Scale the matrix elements
        self.xform_pipeline.view.transformation.right *= scale.x;
        self.xform_pipeline.view.transformation.up *= scale.y;
        self.xform_pipeline.view.transformation.forward *= scale.z;

        self.xform_pipeline.view.scale = scale;

        self.reset_clipping_far_z();
    }
}
