use vek::{Mat4, Vec3, Vec4};

use crate::{graphics::drawing_3d::Point3, math::{matrix::Matrix, vector::Vector}};

impl From<Point3> for Vec4<f32> {
    fn from(value: Point3) -> Self {
        Vec4 {
            x: value.x(),
            y: value.y(), 
            z: value.z(),
            w: 1.0,
        }
    }
}

impl From<Mat4<f32>> for Matrix {
    fn from(value: Mat4<f32>) -> Self {
        let rows = value.into_row_arrays();

        Matrix {
            right:   Vector { x: rows[0][0], y: rows[0][1], z: rows[0][2] },
            up:      Vector { x: rows[1][0], y: rows[1][1], z: rows[1][2] },
            forward: Vector { x: rows[2][0], y: rows[2][1], z: rows[2][2] },
        }
    }
}

impl From<Vec3<f32>> for Vector {
    fn from(value: Vec3<f32>) -> Self {
        Vector { x: value.x, y: value.y, z: value.z }
    }
}

impl From<Matrix> for Mat4<f32> {
    fn from(value: Matrix) -> Self {
        Mat4::<f32>::new(
            value.right.x, value.right.y, value.right.z, 0.0,
            value.up.x,    value.up.y,    value.up.z,    0.0,
            value.forward.x, value.forward.y, value.forward.z, 0.0,
            0.0, 0.0, 0.0, 1.0, // identity W column (no translation)
        )
    }
}