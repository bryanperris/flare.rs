use core::{f32::consts::PI, ops::{Add, Div, Mul, Sub}};

use super::vector::Vector;

#[derive(Debug, Copy, Clone)]
pub struct Angle(pub u16);

impl Default for Angle {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Angle {
    pub const ZERO: Angle = Angle(0);

    pub fn to_rad(self) -> f32 {
        let i = (self.0 >> 8) as u8;
        let f = self.0 as u8;
        let normalized_f = f as f32 / 256.0;
        (i as f32 + normalized_f) * 2.0 * PI / 360.0
    }

    pub fn sin(&self) -> f32 {
        self.to_rad().sin()
    }

    pub fn cos(&self) -> f32 {
        self.to_rad().cos()
    }

    // pub fn asin(&self) -> f32 {
    //     self.to_rad().asin()
    // }

    pub fn acos(v: f32) -> Self {
        let mut vv = (v.abs() * 65536.0).trunc() as i32;

        if vv > 0x10000 {
            return Angle::ZERO;
        }

        Angle(vv as u16).to_rad().acos();

        todo!()
    }

    pub fn atan2(cos: f32, sin: f32) -> Self {
        let mut angle = Angle(0);

        /* Find the smaller of the 2 */
        let q = (sin * sin) + (cos * cos);
        let m = q.sqrt();

        if m == 0.0 {
            return angle;
        }

        if sin.abs() < cos.abs() {
            if cos < 0.0 {
                angle.0 = 0x8000 - (sin / m).asin().trunc() as u16;
            }
            else {
                angle.0 = (sin / m).asin().trunc() as u16;
            }
        }
        else {
            if sin < 0.0 {
                // From D3 it is fixed point 1.0 (0x10000) - value
                angle.0 = 0u16.wrapping_sub((sin / m).acos().trunc() as u16);
            }
            else {
                angle.0 = (sin / m).acos().trunc() as u16;
            }
        }

        angle
    }
}

impl From<u16> for Angle {
    fn from(value: u16) -> Self {
        Angle(value)
    }
}

impl Add for Angle {
    type Output = Angle;

    fn add(self, other: Angle) -> Angle {
        Angle(self.0.wrapping_add(other.0))
    }
}

impl Sub for Angle {
    type Output = Angle;

    fn sub(self, other: Angle) -> Angle {
        Angle(self.0.wrapping_sub(other.0))
    }
}

impl Mul<u16> for Angle {
    type Output = Angle;

    fn mul(self, rhs: u16) -> Angle {
        Angle(self.0.wrapping_mul(rhs))
    }
}

impl Div<u16> for Angle {
    type Output = Angle;

    fn div(self, rhs: u16) -> Angle {
        Angle(self.0.wrapping_div(rhs))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EulerAngle {
    /// X
    pub pitch: Angle,
    /// Y
    pub heading: Angle,
    /// Z
    pub bank: Angle
}

impl EulerAngle {
    pub fn zero_out(&mut self) {
        self.pitch = Angle::ZERO;
        self.heading = Angle::ZERO;
        self.bank = Angle::ZERO;
    }
}

pub type EularAngle = Vector;