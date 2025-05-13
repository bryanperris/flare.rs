use super::{CrossProduct, DotProduct};

#[derive(Debug, Copy, Clone)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32,
}

impl Default for Vector2D {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, }
    }
}

impl CrossProduct for Vector2D {
    type Result = f32;
    
    fn cross(self, rhs: &Self) -> Self::Result {
        (self.x * rhs.y) - (self.y * rhs.x)
    }
}

impl DotProduct for Vector2D {
    fn dot(self, rhs: Self) -> f32 {
        (self.x * rhs.x) + (self.y * rhs.y)
    }
}