pub mod fireball;


use bitflags::bitflags;

use crate::{common::SharedMutRef, create_rng, graphics::bitmap::{videoclip::VideoClip, Bitmap16}, math::vector::Vector, rand::ps_rand};

use super::{
    object::Object, object_dynamic_behavior::MovementType, object_static_behavior::PhysicsFlags,
    room::Room,
};

const MAX_EFFECTS: usize = 5000;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VisualEffectFlags: u32 {
        const NONE               = 0;
        const USES_LIFELEFT      = 1;
        const WINDSHIELD_EFFECT  = 2;
        const DEAD               = 4;
        const PLANAR             = 8;
        const REVERSE            = 16;
        const EXPAND             = 32;
        const ATTACHED           = 64;
        const NO_Z_ADJUST        = 128;
        const LINK_TO_VIEWER     = 256; // Always link into the room that the viewer is in
    }
}

#[derive(Debug, Clone)]
pub struct VisualEffectAttachInfo {
    pub object: Option<SharedMutRef<Object>>,
    pub dest_object: Option<SharedMutRef<Object>>,

    pub model: Option<()>,
    pub start_vert: usize,
    pub end_vert: usize,

    pub subnum: u8,
    pub subnum2: u8,
}

impl Default for VisualEffectAttachInfo {
    fn default() -> Self {
        Self { object: Default::default(), dest_object: Default::default(), model: Default::default(), start_vert: Default::default(), end_vert: Default::default(), subnum: Default::default(), subnum2: Default::default() }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct AxisBillboardInfo {
    pub width: u8,
    pub height: u8,
    pub texture: u8,
}

#[derive(Debug, Clone)]
pub struct ParticleState {
    pub start_position: Vector,
    pub end_position: Vector,

    // XXX: lets use thse similar fields from movement type
    // pub velocity: Vector,
    // pub mass: f32,
    // pub drag: f32,
    // pub physics_flags: PhysicsFlags,

    pub size: f32,
    pub life_left: f32,
    pub life_time: f32,
    pub creation_time: f32,
    pub lighting_color: u16,
    pub movement_type: Option<MovementType>,
    pub attachment: Option<VisualEffectAttachInfo>,
    pub flags: VisualEffectFlags,
    pub resource: Option<CustomResource>
}

#[derive(Debug, Clone)]
pub enum CustomResource {
    Bitmap(SharedMutRef<dyn Bitmap16>),
    VideoClip(SharedMutRef<VideoClip>)
}

impl Default for ParticleState {
    fn default() -> Self {
        Self {
            flags: VisualEffectFlags::NONE,
            start_position: Default::default(),
            end_position: Default::default(),
            size: Default::default(),
            life_left: Default::default(),
            life_time: Default::default(),
            creation_time: 0.0,
            lighting_color: 0,
            movement_type: None,
            attachment: None,
            resource: None
        }
    }
}

pub trait VisualEffect: core::fmt::Debug {
    fn particle_state(&self) -> &ParticleState;
}

#[cfg(not(feature = "dedicated_server"))]
pub fn emit_visual_effect_in_room(room: &mut Room, effect: Box<dyn VisualEffect>) {
    room.visual_effects.push(effect);
}