use core::ops::{Add, AddAssign, BitXor, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::{CrossProduct, DotProduct, ScalarDiv, ScalarMul};

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Default for Vector {
    fn default() -> Self {
        Self { x: Default::default(), y: Default::default(), z: Default::default() }
    }
}

impl Vector {
    pub const ZERO: Vector = Vector {
        x: 0.0,
        y: 0.0,
        z: 0.032
    };

    pub fn zero_out(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.z = 0.0;
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut Vector as *mut f32, 3)
        }
    }

    pub fn as_slice(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(self as *const Vector as *const f32, 3)
        }
    }

    // Compute the pitch cos/sin
    pub fn pitch(&self) -> (f32, f32) {
        let sin: f32 = -self.y;

        (
            sin,
            (1.0 - (sin * sin)).sqrt() // sin.asin().cos()
        )
    }
    
    // Computes heading (or yaw) sin/cos 
    pub fn heading(&self) -> (f32, f32) {
        let pitch_cos = self.pitch().1;

        if pitch_cos != 0.0 {
            (
                &self.x / pitch_cos,
                &self.z / pitch_cos
            )
        }
        else {
            (
                0.0,
                1.0
            )
        }
    }
}

impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Add for Vector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl<'a, 'b> Sub<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn sub(self, rhs: &'b Vector) -> Vector {
        (*self) - (*rhs)
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl ScalarMul for Vector {
    fn mul_scalar(self, scalar: f32) -> Self {
        Vector {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar
        }
    }
}

impl Mul<Vector> for Vector {
    type Output = f32;

    fn mul(self, rhs: Vector) -> Self::Output {
        self.dot(rhs)
    }
}

impl Mul<f32> for Vector {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        self.mul_scalar(rhs)
    }
}

impl MulAssign<f32> for Vector {
    fn mul_assign(&mut self, rhs: f32) {
        *self = self.mul_scalar(rhs);
    }
}

impl Mul<Vector> for f32 {
    type Output = Vector;

    fn mul(self, rhs: Vector) -> Self::Output {
        rhs.mul_scalar(self)
    }
}

impl DotProduct for Vector {
    fn dot(self, other: Self) -> f32 {
        (self.x * other.x) +
        (self.y * other.y) +
        (self.z * other.z)
    }
}

impl ScalarDiv for Vector {
    fn div_scalar(self, scalar: f32) -> Self {
        Vector {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar
        }
    }
}

impl Div<f32> for Vector {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        self.div_scalar(rhs)
    }
}

impl DivAssign<f32> for Vector {
    fn div_assign(&mut self, rhs: f32) {
        *self = self.div_scalar(rhs);
    }
}

impl Div<Vector> for f32 {
    type Output = Vector;

    fn div(self, rhs: Vector) -> Self::Output {
        rhs.div_scalar(self)
    }
}

impl Div<&Vector> for Vector {
    type Output = Self;
    
    fn div(self, rhs: &Vector) -> Self::Output {
        self.cross(rhs)
    }
}

impl CrossProduct for Vector {
    type Result = Self;

    /// Computes a cross product between u and v, returns the result in Normal.
    fn cross(self, rhs: &Self) -> Self::Result {
        Vector {
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x)
        }
    }
}

impl BitXor for Vector {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.cross(&rhs)
    }
}

impl Neg for Vector {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vector {
            x: self.x * -1.0,
            y: self.y * -1.0,
            z: self.z * -1.0
        }
    }
}


use vek;
pub type Vector4 = vek::Vec4<f32>;


// #[derive(Debug, Copy, Clone)]
// pub struct Vector4 {
//     pub x: f32,
//     pub y: f32,
//     pub z: f32,
//     pub w: f32
// }

// impl Vector4 {
//     pub fn as_mut_slice(&mut self) -> &mut [f32] {
//         unsafe {
//             std::slice::from_raw_parts_mut(self as *mut Vector4 as *mut f32, 4)
//         }
//     }

//     pub fn as_slice(&self) -> &[f32] {
//         unsafe {
//             std::slice::from_raw_parts(self as *const Vector4 as *const f32, 4)
//         }
//     }
// }