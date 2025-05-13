use derive_builder::Builder;

use crate::{graphics::{bitmap::videoclip::VideoClip, texture::TextureSizeType}, string::D3String};

use super::{ParticleState, VisualEffect, VisualEffectFlags};

#[derive(Debug, Copy, Clone)]
pub enum FireballEffectType {
    Explosion,
    Smoke,
    Effect,
    Billow,
    Spark
}

#[derive(Debug, Clone)]
pub struct FireballEffectInfo {
    pub filename: Option<D3String>,
    pub effect_type: FireballEffectType,
    pub texture_size: TextureSizeType,
    /// How long this animation should last (in seconds)
    pub total_life: f32,
    /// How big this explosion is
    pub size: f32,
}

#[derive(Debug)]
pub struct FireballEffect {
    pub fireball_info: FireballEffectInfo,
    pub particle_state: ParticleState,
}

impl FireballEffect {
}

impl VisualEffect for FireballEffect {
    fn particle_state(&self) -> &ParticleState {
        todo!()
    }
}