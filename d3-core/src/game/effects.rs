use std::rc::Rc;
use crate::math::vector::Vector;
use super::object::Object;
use bitflags::bitflags;

#[derive(Debug, Clone)]
pub struct FadeEffect {
    pub time: f32,
    pub max_time: f32
}

#[derive(Debug, Clone)]
pub enum FadeEffectType {
    In(FadeEffect),
    Out(FadeEffect)
}

#[derive(Debug, Clone)]
pub struct DamageEffect {
    pub time: f32,
    pub per_second: f32,
    pub last_time: f32,
    pub last_owner: Rc<Object>,
}

#[derive(Debug, Clone)]
pub struct AudibleEffect {
    pub volume_change_time: f32,
    pub volume_old_position: Vector,
    pub volume_old_room: (),
    pub sound_handle: Option<()>,
}

#[derive(Debug, Clone)]
pub struct PowerupEffect {
    pub last_obect_hit_time: f32,
    pub last_object_hit: Option<Rc<Object>>,
}

#[derive(Debug, Clone)]
pub struct SpecularLightEffect {
    pub position: Vector,
    pub magnitude: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct DynamicVolumeLightEffect {
    pub this_frame: (),
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub is_morphing: bool
}

#[derive(Debug, Clone)]
pub struct LiquidEffect {
    pub time_left: f32,
    pub magnitude: u8,
}

#[derive(Debug, Clone)]
pub struct SparkEffect {
    pub delay: f32,
    pub timer: f32,
    pub time_left: f32
}

#[derive(Debug, Clone)]
pub struct FreezeEffect {
    pub scalar: f32
}

#[derive(Debug, Clone)]
pub struct DeformEffect {
    pub time: f32,
    /// How many units to deform when drawing
    pub range: usize
}

#[derive(Debug, Clone)]
pub struct CloakEffect {
    pub time: f32,
    pub wear_off_message: bool
}

#[derive(Debug, Clone)]
pub struct ColoredEffect {
    pub time: f32,
    pub alpha: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Debug, Clone)]
pub struct AttachmentEffect {
    pub attached_object: Rc<Object>
}