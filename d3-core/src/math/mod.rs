use core::{f32, ops::{Add, Mul, Neg, Sub}, panic};

use angle::{Angle, EulerAngle};
use matrix::{Matrix, Matrix4};
use vector::Vector;
use vector2d::Vector2D;

use std::{f32::consts::PI, vec};

pub mod angle;
pub mod matrix;
pub mod vector;
pub mod vector2d;

pub trait DotProduct {
    fn dot(self, rhs: Self) -> f32;
}

pub trait CrossProduct {
    type Result;
    fn cross(self, rhs: &Self) -> Self::Result;
}

pub trait ScalarMul {
    fn mul_scalar(self, scalar: f32) -> Self;
}

pub trait ScalarDiv {
    fn div_scalar(self, scalar: f32) -> Self;
}

/// Given a 3space triplet, computes the u,v coords for a texture map at that position
/// on a sphere.
pub fn calc_sphere_map(x: f32, y: f32, z: f32, radius: f32, h: i32) -> (f32, f32) {
    debug_assert!(y < 0.0);

    let h = h as f32;

    // Produces u from 0 to 1
    let u = h / (65535.0 / 16.005); // account for floating point precision error

    let mut v = (y / radius).acos() / (f32::consts::PI / 2.0); // v=.5 to 1
    v = v / 0.5; // v=1 to 2
    v = v - 1.0; // v=0 to 1

    (u, v)
}

impl Mul<Vector> for Matrix {
    type Output = Vector;

    fn mul(self, rhs: Vector) -> Self::Output {
        Vector {
            x: self.right.dot(rhs),
            y: self.up.dot(rhs),
            z: self.forward.dot(rhs),
        }
    }
}

impl Mul<Matrix> for Vector {
    type Output = Vector;

    fn mul(self, rhs: Matrix) -> Self::Output {
        Vector {
            x: rhs.right.dot(self),
            y: rhs.up.dot(self),
            z: rhs.forward.dot(self),
        }
    }
}

impl<'a> Mul<&'a Matrix> for Vector {
    type Output = Vector;

    fn mul(self, rhs: &'a Matrix) -> Self::Output {
        *rhs * self
    }
}

impl Angle {
    pub fn into_x_rotation(&self) -> Matrix {
        Matrix::new_rotation_x(self.sin(), self.cos())
    }

    pub fn into_y_rotation(&self) -> Matrix {
        Matrix::new_rotation_y(self.sin(), self.cos())
    }

    pub fn into_z_rotation(&self) -> Matrix {
        Matrix::new_rotation_z(self.sin(), self.cos())
    }

    pub fn new_random() -> Self {
        extern crate tinyrand;
        use tinyrand::{Rand, StdRand};
   
        let mut rand = crate::create_rng();

        Angle(rand.next_u16())
    }
}

impl Vector2D {
    pub fn magnitude(vector: &Vector2D) -> f32 {
        // TODO: SSE for 2D vec
        // #[cfg(target_feature = "sse")]
        // {
        //     if std::arch::is_x86_feature_detected!("sse") {
        //         return Vector::magnitude_sse(vector);
        //     }
        // }

        vector.dot(*vector).sqrt()
    }
}

impl Vector {
    pub fn average(vector: &mut Vector, num: i32) {
        assert!(num != 0);

        let result = vector.div_scalar(num as f32);
        vector.x = result.x;
        vector.y = result.y;
        vector.z = result.z;
    }

    pub fn add_vectors(result: &mut Vector, a: &Vector, b: &Vector) {
        let sum = a.add(*b);
        result.x = sum.x;
        result.y = sum.y;
        result.z = sum.z;
    }

    pub fn sub_vectors(result: &mut Vector, a: &Vector, b: &Vector) {
        let diff = a.sub(b);
        result.x = diff.x;
        result.y = diff.y;
        result.z = diff.z;
    }

    pub fn magnitude(vector: &Vector) -> f32 {
        #[cfg(target_feature = "sse")]
        {
            if std::arch::is_x86_feature_detected!("sse") {
                return Vector::magnitude_sse(vector);
            }
        }

        vector.dot(*vector).sqrt()
    }

    #[cfg(target_feature = "sse")]
    fn magnitude_sse(vector: &Vector) -> f32 {
        use std::arch::x86_64::*;

        unsafe {
            // Load the vector components into an SSE register
            let v = _mm_set_ps(0.0, vector.z, vector.y, vector.x);

            // Perform element-wise multiplication (v * v)
            let v2 = _mm_mul_ps(v, v);

            // Horizontal add to sum the squares
            let shuf = _mm_movehdup_ps(v2); // (x2, x2, y2, y2)
            let sums = _mm_add_ps(v2, shuf); // (x2 + y2, x2 + y2, y2 + x2, y2 + x2)
            let shuf = _mm_movehl_ps(shuf, sums); // (x2 + y2, x2 + y2, z2, z2)
            let sums = _mm_add_ss(sums, shuf); // (x2 + y2 + z2, ...)

            // Extract the final result and take the square root
            _mm_cvtss_f32(_mm_sqrt_ss(sums))
        }
    }

    pub fn distance(a: &Vector, b: &Vector) -> f32 {
        Vector::magnitude(&a.sub(b))
    }

    pub fn normalize(vector: &mut Vector) -> f32 {
        let mag = Vector::magnitude(vector);

        if mag > 0.0 {
            vector.div_scalar(mag);
            mag
        } else {
            vector.x = 1.0;
            vector.y = Vector::ZERO.y;
            vector.z = Vector::ZERO.z;
            0.0
        }
    }

    /// Calculates the perpendicular vector given three points
    /// Parms:	n - the computed perp vector (filled in)
    /// v0,v1,v2 - three clockwise vertices
    pub fn compute_perpendicular_vector(result: &mut Vector, a: &Vector, b: &Vector, c: &Vector) {
        let x = b.sub(a);
        let y = c.sub(b);
        let r = x.cross(&y);
        result.x = r.x;
        result.y = r.y;
        result.z = r.z;
    }

    // Calculates the (normalized) surface normal give three points
    // Parms:	n - the computed surface normal (filled in)
    //			v0,v1,v2 - three clockwise vertices
    // Returns the magnitude of the normal before it was normalized.
    // The bigger this value, the better the normal.
    pub fn compute_normal_vector(result: &mut Vector, a: &Vector, b: &Vector, c: &Vector) -> f32 {
        Vector::compute_perpendicular_vector(result, a, b, c);
        Vector::normalize(result)
    }

    // Returns a normalized direction vector between two points
    // Just like vm_GetNormalizedDir(), but uses sloppier magnitude, less precise
    // Parameters:	dest - filled in with the normalized direction vector
    //					start,end - the start and end points used to calculate the vector
    // Returns:		the distance between the two input points
    pub fn compute_normalized_direction(result: &mut Vector, end: &Vector, start: &Vector) -> f32 {
        let diff = end.sub(start);
        result.x = diff.x;
        result.y = diff.y;
        result.z = diff.z;
        Vector::normalize(result)
    }

    pub fn multiply_vec_by_transpose(result: &mut Vector, vector: &Vector, matrix: &Matrix) {
        result.x = vector.dot(Vector {
            x: matrix.right.x,
            y: matrix.up.x,
            z: matrix.forward.x,
        });
        result.y = vector.dot(Vector {
            x: matrix.right.y,
            y: matrix.up.y,
            z: matrix.forward.y,
        });
        result.z = vector.dot(Vector {
            x: matrix.right.z,
            y: matrix.up.z,
            z: matrix.forward.z,
        });
    }

    // Computes the distance from a point to a plane.
    // Parms:	checkp - the point to check
    // Parms:	norm - the (normalized) surface normal of the plane
    //				planep - a point on the plane
    // Returns:	The signed distance from the plane; negative dist is on the back of the plane
    pub fn compute_distance_to_plane(
        check_point: &Vector,
        normal: &Vector,
        plane_point: &Vector,
    ) -> f32 {
        return check_point.sub(plane_point).dot(*normal);
    }

    pub fn compute_slope_2d(p1: &Vector, p2: &Vector) -> f32 {
        if p2.y - p1.y == 0.0 {
            return 0.0;
        }

        (p2.x - p1.x) / (p2.y - p1.y)
    }

    /// Computes a matrix from a vector and and angle of rotation around that vector
    /// Parameters:	    vector - direction
    //					angle - the angle of rotation around the vector
    pub fn compute_vector_angle_matrix(vector: &Vector, angle: &Angle) -> Matrix {
        let pitch = vector.pitch();
        let heading = vector.heading();

        Matrix::new_rotation_x(pitch.0, pitch.1)
            * Matrix::new_rotation_y(heading.0, heading.1)
            * angle.into_z_rotation()
    }
    
    /// Computes the delta angle between two vectors
    /// Vector inputs must be normalized
    pub fn compute_delta_angle(v0_n: &Vector, v1_n: &Vector, fvec: Option<&Vector>) -> Angle {
        let mut angle = Angle::acos(v0_n.dot(*v1_n));

        if fvec.is_some() {
            let t = v0_n.cross(&v1_n);

            if t.dot(fvec.unwrap().clone()) < 0.0 {
                angle = Angle(0-angle.0);
            }
        }

        angle
    }

    /// Computes the real center of a given polygon
    pub fn compute_centroid(&self, vecs: &[Vector]) -> (Vector, f32) {
        assert!(vecs.len() > 2);

        let mut centroid = Vector::ZERO;
        let mut normal = Vector::ZERO;

        Vector::compute_perpendicular_vector(
            &mut normal, 
            &vecs[0],
            &vecs[1],
            &vecs[2]);

        let normal_cloned = normal.clone();

        // First figure out the total area of this polygon
        let mut total_area = Vector::magnitude(&normal) / 2.0;
        let mut area = total_area;

        for i in 0..(vecs.len() - 1) {
            Vector::compute_perpendicular_vector(
                &mut normal, 
                &vecs[0],
                &vecs[i],
                &vecs[i + 1]);

            total_area += Vector::magnitude(&normal) / 2.0;
        }

        // Now figure out how much weight each triangle represents to the overall
        // polygon
        let mut temp_center = Vector::ZERO;

        for i in 0..3 {
            temp_center = temp_center + vecs[i];
        }

        temp_center = temp_center / 3.0;

        centroid = centroid + (temp_center * (area / total_area));

        /* Do the same for the rest */
        for i in 2..(vecs.len() - 1) {
            Vector::compute_perpendicular_vector(
                &mut normal, 
                &vecs[0],
                &vecs[i],
                &vecs[i + 1]);

            area = Vector::magnitude(&normal) / 2.0;

            temp_center = Vector::ZERO;

            temp_center = temp_center + vecs[0];
            temp_center = temp_center + vecs[i];
            temp_center = temp_center + vecs[i + 1];

            temp_center = temp_center / 3.0;

            centroid = centroid + (temp_center * (area / total_area));
        }

        (centroid, total_area)
    }

    pub fn new_random() -> Self {
        extern crate tinyrand;
        use tinyrand::{Rand, StdRand};
   
        let mut rand = crate::create_rng();

        Vector {
            x: (rand.next_u32() as i32 - i32::MAX / 2) as f32,
            y: (rand.next_u32() as i32 - i32::MAX / 2) as f32,
            z: (rand.next_u32() as i32 - i32::MAX / 2) as f32,
        }
    }

    // Given a set of points, computes the minimum bounding sphere of those points
    pub fn compute_bounding_sphere(center: &mut Vector, vecs: &[Vector]) -> f32 {
        let mut min_x = &vecs[0];
        let mut max_x = &vecs[0];
        let mut min_y = &vecs[0];
        let mut max_y = &vecs[0];
        let mut min_z = &vecs[0];
        let mut max_z = &vecs[0];

        // First, find the points with the min & max x,y, & z coordinates
        for i in 0..vecs.len() {
            let vec_ref = &vecs[i];

            if vec_ref.x < min_x.x {
                min_x = vec_ref;
            }

            if vec_ref.x > max_x.x {
                max_x = vec_ref;
            }

            if vec_ref.y < min_y.y {
                min_y = vec_ref;
            }

            if vec_ref.y > max_y.y {
                max_y = vec_ref;
            }

            if vec_ref.z < min_z.z {
                min_z = vec_ref;
            }

            if vec_ref.z > max_z.z {
                max_z = vec_ref;
            }
        }

        /* Calculate intial sphere */
        let dx = Vector::distance(min_x, max_x);
        let dy = Vector::distance(min_y, max_y);
        let dz = Vector::distance(min_z, max_z);

        let mut rad: f32;

        if dx > dy {
            if dx > dz {
                *center = (*min_x + *max_x) / 2.0;
                rad = dx / 2.0;
            }
            else {
                *center = (*min_z + *max_z) / 2.0;
                rad = dz / 2.0;
            }
        }
        else if dy > dz {
            *center = (*min_y + *max_y) / 2.0;
            rad = dy / 2.0;
        }
        else {
            *center = (*min_z + *max_z) / 2.0;
            rad = dz / 2.0;
        }

        // Go through all points and look for ones that don't fit
        let mut rad2 = rad * rad;
        for i in 0..vecs.len() {
            let vec_ref = &vecs[i];
            let delta = vec_ref.clone() - *center;
            let t2 = delta.x * delta.x + delta.y * delta.y + delta.z * delta.z;

            // If point outside, make the sphere bigger
            if t2 > rad2 {
                let t = t2.sqrt();
                rad = (rad + t) / 2.0;
                rad2 = rad * rad;
                *center = *center * (t - rad) / t;
            }
        }

        rad
    }
}

impl Mul<Matrix> for Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Matrix) -> Self::Output {
        let mut m = Matrix::IDENTITY;

        m.right.x = rhs.right.dot(Vector {
            x: self.right.x,
            y: self.up.x,
            z: self.forward.x,
        });
        m.up.x = rhs.up.dot(Vector {
            x: self.right.x,
            y: self.up.x,
            z: self.forward.x,
        });
        m.forward.x = rhs.forward.dot(Vector {
            x: self.right.x,
            y: self.up.x,
            z: self.forward.x,
        });

        m.right.y = rhs.right.dot(Vector {
            x: self.right.y,
            y: self.up.y,
            z: self.forward.y,
        });
        m.up.y = rhs.up.dot(Vector {
            x: self.right.y,
            y: self.up.y,
            z: self.forward.y,
        });
        m.forward.y = rhs.forward.dot(Vector {
            x: self.right.y,
            y: self.up.y,
            z: self.forward.y,
        });

        m.right.z = rhs.right.dot(Vector {
            x: self.right.z,
            y: self.up.z,
            z: self.forward.z,
        });
        m.up.z = rhs.up.dot(Vector {
            x: self.right.z,
            y: self.up.z,
            z: self.forward.z,
        });
        m.forward.z = rhs.forward.dot(Vector {
            x: self.right.z,
            y: self.up.z,
            z: self.forward.z,
        });

        m
    }
}

impl Matrix {
    pub fn orthogonalize(&self) -> Matrix {
        let mut m = self.clone();

        assert!(Vector::normalize(&mut m.forward) != 0.0);

        // compute right vector from forward and up
        m.right = m.up.cross(&m.forward);

        if Vector::normalize(&mut m.right) != 0.0 {
            return Matrix::from_vector(Some(&m.forward), None, None);
        }

        // in case it wasn't entirely perpendicular
        m.up = m.forward.cross(&m.right);

        m
    }

    pub fn compute_rotation_3d(angle: &EulerAngle) -> Matrix {
        angle.pitch.into_x_rotation()
            * angle.heading.into_y_rotation()
            * angle.bank.into_z_rotation()
    }

    pub fn from_vector(
        forward: Option<&Vector>,
        up: Option<&Vector>,
        right: Option<&Vector>,
    ) -> Matrix {
        let mut m = Matrix::ZERO;
        let mut t = Matrix::ZERO;

        if forward.is_none() {
            if up.is_some() {
                Matrix::vector_to_matrix(&mut t, up, None, right);
                m.forward = -t.up;
                m.up = t.forward;
                m.right = t.right;
            } else {
                Matrix::vector_to_matrix(&mut t, right, None, None);
                m.forward = -t.right;
                m.up = t.up;
                m.right = t.forward;
            }
        } else {
            assert!(up.is_some() && right.is_some());
            Matrix::vector_to_matrix(&mut m, forward, up, right);
        }

        m
    }

    fn vector_to_matrix_forward_only(xvec: &mut Vector, yvec: &mut Vector, zvec: &mut Vector) {
        if zvec.x == 0.0 && zvec.z == 0.0 {
            // forward vec is straight up or down
            xvec.x = 1.0;
            yvec.z = if zvec.y < 0.0 { 1.0 } else { -1.0 };
            xvec.y = 0.0;
            xvec.z = 0.0;
            yvec.x = 0.0;
            yvec.y = 0.0;
        } else {
            // not straight up or down
            xvec.x = zvec.z;
            xvec.y = 0.0;
            xvec.z = -zvec.x;

            let _ = Vector::normalize(xvec);

            *yvec = zvec.cross(&xvec);
        }
    }

    fn vector_to_matrix(
        matrix: &mut Matrix,
        f: Option<&Vector>,
        u: Option<&Vector>,
        r: Option<&Vector>,
    ) {
        let xvec = &mut matrix.right;
        let yvec = &mut matrix.up;
        let zvec = &mut matrix.forward;

        assert!(f.is_some());

        *zvec = f.unwrap().clone();

        assert!(Vector::normalize(zvec) != 0.0);

        if u.is_none() {
            if r.is_none() {
                // Just forward vec
                Matrix::vector_to_matrix_forward_only(xvec, yvec, zvec);
            } else {
                // Use right vec
                *xvec = r.unwrap().clone();

                *yvec = zvec.cross(&xvec);

                if Vector::normalize(xvec) == 0.0 {
                    Matrix::vector_to_matrix_forward_only(xvec, yvec, zvec);
                }

                // in case it wasn't entirely perpendicular
                *xvec = yvec.cross(&zvec);
            }
        } else {
            // Use up vec
            *yvec = u.unwrap().clone();

            *xvec = yvec.cross(&zvec);

            if Vector::normalize(yvec) == 0.0 {
                Matrix::vector_to_matrix_forward_only(xvec, yvec, zvec);
            }

            // in case it wasn't entirely perpendicular
            *yvec = zvec.cross(&xvec);
        }
    }

    pub fn into_euler(&self) -> EulerAngle {
        let mut angles = EulerAngle {
            bank: Angle::ZERO,
            heading: Angle::ZERO,
            pitch: Angle::ZERO
        };

        // Deal with straight up or straight down
        if self.forward.x.abs() < f32::EPSILON && self.forward.z.abs() < f32::EPSILON {
            angles.pitch = if self.forward.y > 0.0 {
                Angle(0xC000)
            }
            else {
                Angle(0x4000)
            };

            angles.bank = Angle::ZERO;
            angles.heading = Angle::atan2(self.right.z, self.right.z);

            return angles;
        }

        angles.heading = Angle::atan2(self.forward.z, self.forward.x);

        let heading_sin = angles.heading.sin();
        let heading_cos = angles.heading.cos();

        let pitch_cos: f32;
        if heading_sin.abs() > heading_cos.abs() {
            pitch_cos = self.forward.x / heading_sin;
        }
        else {
            pitch_cos = self.forward.z / heading_cos;
        }

        angles.pitch = Angle::atan2(pitch_cos, -self.forward.y);

        let bank_sin = self.right.y / pitch_cos;
        let bank_cos = self.up.y / pitch_cos;

        angles.bank = Angle::atan2(bank_cos, bank_sin);

        angles
    }

    pub fn compute_determinant(&self) -> f32 {
        self.right.x * self.up.y * self.forward.z - self.right.x * self.up.z * self.forward.y -
        self.right.y * self.up.x * self.forward.z + self.right.y * self.up.z * self.forward.x +
        self.right.z * self.up.x * self.forward.y - self.right.z * self.up.y * self.forward.x
    }
}
