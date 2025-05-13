use core::ops::{Add, Div, Mul, Neg, Sub};

use super::{vector::Vector, ScalarDiv, ScalarMul};

macro_rules! swap {
    ($a:expr, $b:expr) => {
        {
            let temp = $a;
            $a = $b;
            $b = temp;
        }
    };
}

#[derive(Debug, Copy, Clone)]
pub struct Matrix {
    pub right: Vector,
    pub up: Vector,
    pub forward: Vector
}

impl Default for Matrix {
    fn default() -> Self {
        Self { right: Default::default(), up: Default::default(), forward: Default::default() }
    }
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix {
        right:   Vector { x: 1.0, y: 0.0, z: 0.0 },
        up:      Vector { x: 0.0, y: 1.0, z: 0.0 },
        forward: Vector { x: 0.0, y: 0.0, z: 1.0 },
    };

    pub const INVERSE: Matrix = Matrix {
        right:   Vector { x: -1.0, y: 0.0,  z: 0.0 },
        up:      Vector { x: 0.0,  y: -1.0, z: 0.0 },
        forward: Vector { x: 0.0,  y: 0.0,  z: -1.0 },
    };

    pub const ZERO: Matrix = Matrix {
        right:   Vector { x: 0.0, y: 0.0, z: 0.0 },
        up:      Vector { x: 0.0, y: 0.0, z: 0.0 },
        forward: Vector { x: 0.0, y: 0.0, z: 0.0 },
    };

    pub fn into_transposed(self) -> Self {
        self.transpose()
    }

    pub fn transpose(&self) -> Self {
        let mut m = self.clone();
        swap!(m.right.z, m.forward.x);
        swap!(m.up.x, m.right.y);
        swap!(m.forward.y, m.up.z);
        m
    }

    pub fn zero_out(&mut self) {
        self.right = Vector::ZERO;
        self.up = Vector::ZERO;
        self.forward = Vector::ZERO;
    }

    pub fn new_rotation_x(sin: f32, cos: f32) -> Matrix {
        Matrix {
            right:   Vector { x: 1.0, y: 0.0, z: 0.0 },
            up:      Vector { x: 0.0, y: cos, z: -sin },
            forward: Vector { x: 0.0, y: sin, z: cos }
        }
    }

    pub fn new_rotation_y(sin: f32, cos: f32) -> Matrix {
        Matrix {
            right:   Vector { x: cos,  y: 0.0, z: sin },
            up:      Vector { x: 0.0,  y: 1.0, z: 0.0 },
            forward: Vector { x: -sin, y: 0.0, z: cos }
        }
    }

    pub fn new_rotation_z(sin: f32, cos: f32) -> Matrix {
        Matrix {
            right:   Vector { x: cos, y: -sin, z: 0.0 },
            up:      Vector { x: sin, y: cos,  z: 0.0 },
            forward: Vector { x: 0.0, y: 0.0,  z: 1.0 }
        }
    }
}

impl Add for Matrix {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Matrix {
            right: self.right + rhs.right,
            up: self.up + rhs.up,
            forward: self.forward + rhs.forward
        }
    }
}

impl Sub for Matrix {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Matrix {
            right: self.right - rhs.right,
            up: self.up - rhs.up,
            forward: self.forward - rhs.forward
        }
    }
}

impl ScalarMul for Matrix {
    fn mul_scalar(self, scalar: f32) -> Self {
        Matrix {
            right: self.right.mul_scalar(scalar),
            up: self.up.mul_scalar(scalar),
            forward: self.forward.mul_scalar(scalar)
        }
    }
}

impl Mul<f32> for Matrix {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        self.mul_scalar(rhs)
    }
}

impl Mul<Matrix> for f32 {
    type Output = Matrix;

    fn mul(self, rhs: Matrix) -> Self::Output {
        rhs.mul_scalar(self)
    }
}

impl ScalarDiv for Matrix {
    fn div_scalar(self, scalar: f32) -> Self {
        Matrix {
           right: self.right.div_scalar(scalar),
           up: self.up.div_scalar(scalar),
           forward: self.forward.div_scalar(scalar)
        }
    }
}

impl Div<f32> for Matrix {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        self.div_scalar(rhs)
    }
}

impl Div<Matrix> for f32 {
    type Output = Matrix;

    fn div(self, rhs: Matrix) -> Self::Output {
        rhs.div_scalar(self)
    }
}

use vek;
pub type Matrix4 = vek::Mat4<f32>;