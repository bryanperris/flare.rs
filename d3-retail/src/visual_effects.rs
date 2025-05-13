use std::collections::HashMap;

use d3_core::game::visual_effects::fireball::{FireballEffect, FireballEffectInfo, FireballEffectType};
use d3_core::game::object_dynamic_behavior::MovementType;
use d3_core::game::object_static_behavior::{Drawable, Physical, PhysicsFlags};
use d3_core::game::prelude::*;
use d3_core::game::room::Room;
use d3_core::game::visual_effects::{ParticleState, VisualEffectFlags};
use d3_core::graphics::rendering::{AlphaType, AlphaTypeFlags, ColorModelType, LightStateType, OverlayTextureType, Renderer, TextureType};
use d3_core::graphics::DrawableResource;
use d3_core::{create_rng, gr_16_to_color, gr_color_blue, gr_color_green, gr_color_red, gr_rgb, gr_rgb16};
use d3_core::graphics::bitmap::Bitmap16;
use d3_core::graphics::procedural::FireEmitterType;
use d3_core::graphics::texture::TextureSizeType;
use d3_core::rand::ps_rand;
use d3_core::{game::visual_effects::VisualEffect, math::vector::Vector, math};
use once_cell::sync::Lazy;
use tinyrand::Rand;

use d3_core::graphics::ddgr_color;

use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetailFireballEffectType {
    MedExplosion2,
    SmallExplosion2,
    MedExplosion,
    MedExplosion3,
    BigExplosion,
    Billowing,
    SmallExplosion,
    MedSmoke,
    BlackSmoke,
    BlastRing,
    SmokeTrail,
    CustomExplosion,
    ShrinkingBlast,
    Smoldering,
    ShrinkingBlast2,
    HotSpark,
    CoolSpark,
    GradientBall,
    Spray,
    FadingLine,
    MuzzleFlash,
    ShipHit,
    BlueBlastRing,
    Particle,
    Afterburner,
    NapalmBall,
    LightningOriginA,
    LightningOriginB,
    Raindrop,
    PuddleDrop,
    GravityField,
    LightningBolt,
    InvulHit,
    SineWave,
    AxisBillboard,
    DefaultCorona,
    HeadlightCorona,
    StarCorona,
    SunCorona,
    Snowflake,
    ThickLightning,
    BlueFire,
    Rubble1,
    Rubble2,
    WaterSplash,
    Shatter,
    Shatter2,
    BillboardSmokeTrail,
    MassDriverEffect,
    BlueExplosion,
    GraySpark,
    GrayLightningBolt,
    MercBossMassDriverEffect,
}

fn new_fireball_effect(
    filename: D3String,
    eff_type: FireballEffectType,
    tex_size: TextureSizeType,
    lifetime: f32,
    size: f32,
) -> FireballEffectInfo {
    FireballEffectInfo {
        filename: Some(filename),
        effect_type: eff_type,
        texture_size: tex_size,
        total_life: lifetime,
        size: size,
    }
}

fn new_fireball_effect_no_filename(
    eff_type: FireballEffectType,
    tex_size: TextureSizeType,
    lifetime: f32,
    size: f32,
) -> FireballEffectInfo {
    FireballEffectInfo {
        filename: None,
        effect_type: eff_type,
        texture_size: tex_size,
        total_life: lifetime,
        size: size,
    }
}

static FIREBALL_LUT: Lazy<HashMap<RetailFireballEffectType, FireballEffectInfo>> =
    Lazy::new(|| {
        HashMap::from([
            (
                RetailFireballEffectType::MedExplosion2,
                new_fireball_effect(
                    "ExplosionAA.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::SmallExplosion2,
                new_fireball_effect(
                    "ExplosionBB.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::MedExplosion,
                new_fireball_effect(
                    "explosionCC.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::MedExplosion3,
                new_fireball_effect(
                    "explosionDD.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::BigExplosion,
                new_fireball_effect(
                    "ExplosionE.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::Billowing,
                new_fireball_effect(
                    "ExplosionFF.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::SmallExplosion,
                new_fireball_effect(
                    "explosionG.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::MedSmoke,
                new_fireball_effect(
                    "smokepuff.oaf".into(),
                    FireballEffectType::Smoke,
                    TextureSizeType::Small,
                    0.7,
                    0.7,
                ),
            ),
            (
                RetailFireballEffectType::BlackSmoke,
                new_fireball_effect(
                    "black_smoke.oaf".into(),
                    FireballEffectType::Smoke,
                    TextureSizeType::Small,
                    0.7,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::BlastRing,
                new_fireball_effect(
                    "BlastRingOrange.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::SmokeTrail,
                new_fireball_effect(
                    "smokepuff.oaf".into(),
                    FireballEffectType::Smoke,
                    TextureSizeType::Small,
                    0.4,
                    0.7,
                ),
            ),
            (
                RetailFireballEffectType::CustomExplosion,
                new_fireball_effect(
                    "smokepuff.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::ShrinkingBlast,
                new_fireball_effect(
                    "explosionblast2.ogf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Normal,
                    0.7,
                    0.7,
                ),
            ),
            (
                RetailFireballEffectType::Smoldering,
                new_fireball_effect(
                    "black_smoke.oaf".into(),
                    FireballEffectType::Smoke,
                    TextureSizeType::Small,
                    0.7,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::ShrinkingBlast2,
                new_fireball_effect(
                    "warp.oaf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::HotSpark,
                new_fireball_effect(
                    "Hotspark.ogf".into(),
                    FireballEffectType::Spark,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::CoolSpark,
                new_fireball_effect(
                    "Coolspark.ogf".into(),
                    FireballEffectType::Spark,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::GradientBall,
                new_fireball_effect(
                    "thrustball.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::Spray,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::FadingLine,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::MuzzleFlash,
                new_fireball_effect(
                    "muzzleflash.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::ShipHit,
                new_fireball_effect(
                    "shiphit.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::BlueBlastRing,
                new_fireball_effect(
                    "BlastRingBlue.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::Particle,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    0.7,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::Afterburner,
                new_fireball_effect(
                    "explosion.oaf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::NapalmBall,
                new_fireball_effect_no_filename(
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::LightningOriginA,
                new_fireball_effect(
                    "LightningOriginA.ogf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::LightningOriginB,
                new_fireball_effect(
                    "LightningOriginB.ogf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Raindrop,
                new_fireball_effect(
                    "Raindrop.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::PuddleDrop,
                new_fireball_effect(
                    "Puddle.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::GravityField,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::LightningBolt,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::InvulHit,
                new_fireball_effect(
                    "InvulnerabilityHit.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::SineWave,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::AxisBillboard,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::DefaultCorona,
                new_fireball_effect(
                    "StarFlare6.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::HeadlightCorona,
                new_fireball_effect(
                    "HeadlightFlare.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::StarCorona,
                new_fireball_effect(
                    "StarFlare.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::SunCorona,
                new_fireball_effect(
                    "SunFlare.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Normal,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Snowflake,
                new_fireball_effect(
                    "Whiteball.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::ThickLightning,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::BlueFire,
                new_fireball_effect(
                    "NapalmFire.oaf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Rubble1,
                new_fireball_effect(
                    "Rocklette1.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Rubble2,
                new_fireball_effect(
                    "Rocklette2.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::WaterSplash,
                new_fireball_effect(
                    "Whiteball.ogf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Shatter,
                new_fireball_effect(
                    "lg.oaf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::Shatter2,
                new_fireball_effect(
                    "lg.oaf".into(),
                    FireballEffectType::Effect,
                    TextureSizeType::Small,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::BillboardSmokeTrail,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::MassDriverEffect,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
            (
                RetailFireballEffectType::BlueExplosion,
                new_fireball_effect(
                    "ExplosionBlkShrk.oaf".into(),
                    FireballEffectType::Explosion,
                    TextureSizeType::Small,
                    0.9,
                    3.0,
                ),
            ),
            (
                RetailFireballEffectType::GraySpark,
                new_fireball_effect(
                    "Coolspark.ogf".into(),
                    FireballEffectType::Spark,
                    TextureSizeType::Small,
                    1.0,
                    1.0,
                ),
            ),
            (
                RetailFireballEffectType::GrayLightningBolt,
                new_fireball_effect_no_filename(
                    FireballEffectType::Effect,
                    TextureSizeType::Tiny,
                    1.0,
                    2.0,
                ),
            ),
        ])
    });

fn new_random_velocity(offset: u32, force_scalar: f32, rand: &mut impl Rand) -> Vector {
    let mut vel = Vector {
        x: ((ps_rand(rand) % 100) - 50) as f32,
        y: (ps_rand(rand) % 100) as f32,
        z: ((ps_rand(rand) % 100) - 50) as f32,
    };

    let _ = Vector::normalize(&mut vel);

    vel = vel * (offset + (ps_rand(rand) % 10)) as f32;
    vel = vel * force_scalar;

    vel
}

#[cfg(not(feature = "dedicated_server"))]
pub fn retail_visual_effect_emit_random_line_sparks(
    gametime: f32,
    num_sparks: usize,
    position: &Vector,
    room: &mut Room,
    color: u16,
    force_scalar: f32,
) {
    let num_sparks = num_sparks * 2;

    let mut rand = d3_core::create_rng();

    let life = 1.0 + ((ps_rand(&mut rand) % 10) as f32 * 0.15);

    let vis = FireballEffect {
        fireball_info: FIREBALL_LUT
            .get(&RetailFireballEffectType::FadingLine)
            .unwrap()
            .clone(),

        particle_state: ParticleState {
            movement_type: Some(MovementType::Physical(Physical {
                mass: 500.0,
                drag: 0.001,
                flags: PhysicsFlags::GRAVITY | PhysicsFlags::NO_COLLIDE,
                velocity: new_random_velocity(20, force_scalar, &mut rand),
                ..Default::default()
            })),
            size: 0.7 + ((ps_rand(&mut rand) % 10) as f32 * 0.04),
            flags: VisualEffectFlags::USES_LIFELEFT,
            life_time: life,
            life_left: life,
            creation_time: gametime,
            lighting_color: if color == 0 { gr_rgb16!(200 + (ps_rand(&mut rand) % 50), 150 + (ps_rand(&mut rand) % 50), ps_rand(&mut rand) % 50) } else { color },
            ..Default::default()
        }
    };

    room.visual_effects.push(Box::new(vis));
}

#[cfg(not(feature = "dedicated_server"))]
pub fn retail_visual_effect_emit_random_sparks(
    gametime: f32,
    num_sparks: usize,
    position: &Vector,
    room: &mut Room,
    color: u16,
    force_scalar: f32,
) {
    let num_sparks = num_sparks * 2;

    let mut rand = d3_core::create_rng();

    // Create sparks
    for _ in 0..num_sparks {
        let fireball_type = if (ps_rand(&mut rand) % 2) != 0 {
            FIREBALL_LUT
                .get(&RetailFireballEffectType::HotSpark)
                .expect("not hot spark effect found")
                .clone()
        } else {
            FIREBALL_LUT
                .get(&RetailFireballEffectType::CoolSpark)
                .expect("no cool spark effect found")
                .clone()
        };

        let life = 1.0 + ((ps_rand(&mut rand) % 10) as f32 * 0.15);

        let vis = FireballEffect {
            fireball_info: fireball_type,
    
            particle_state: ParticleState {
                movement_type: Some(MovementType::Physical(Physical {
                    mass: 100.0,
                    drag: 0.1,
                    flags: PhysicsFlags::GRAVITY | PhysicsFlags::NO_COLLIDE,
                    velocity: new_random_velocity(10, force_scalar, &mut rand),
                    ..Default::default()
                })),
                size: 0.2 + ((ps_rand(&mut rand) % 10) as f32 * 0.01),
                flags: VisualEffectFlags::USES_LIFELEFT,
                life_time: life,
                life_left: life,
                creation_time: gametime,
                ..Default::default()
            },
        };

        room.visual_effects.push(Box::new(vis));
    }
}

#[cfg(not(feature = "dedicated_server"))]
pub fn retail_visual_effect_emit_random_particles(gametime: f32, num_sparks: usize, position: Vector, room: &mut Room, bitmap: SharedMutRef<dyn Bitmap16>, size: f32, life: f32) {
    let tenth_life = life / 10.0;
    let tenth_size = size / 10.0;

    let mut rand = create_rng();

    for _ in 0..num_sparks {
        let life = life + (((ps_rand(&mut rand) % 11) - 5) as f32 * tenth_life);

        let vis = FireballEffect {
            fireball_info: FIREBALL_LUT
            .get(&RetailFireballEffectType::Particle)
            .unwrap()
            .clone(),
    
            particle_state: ParticleState {
                movement_type: Some(MovementType::Physical(Physical {
                    mass: 100.0,
                    drag: 0.1,
                    flags: PhysicsFlags::GRAVITY | PhysicsFlags::NO_COLLIDE,
                    velocity: new_random_velocity(10, 1.0, &mut rand),
                    ..Default::default()
                })),
                size: size + ((ps_rand(&mut rand) % 10) as f32 * tenth_size),
                flags: VisualEffectFlags::USES_LIFELEFT,
                life_time: life,
                life_left: life,
                creation_time: gametime,
                ..Default::default()
            },
        };

        room.visual_effects.push(Box::new(vis));
    }
}

#[derive(Debug)]
pub struct RetailFireballEffect {
    pub effect_type: RetailFireballEffectType,
    pub fireball: FireballEffect
}

impl VisualEffect for RetailFireballEffect {
    fn particle_state(&self) -> &ParticleState {
        self.fireball.particle_state()
    }
}

impl DrawableResource for RetailFireballEffect {
    fn draw_to_renderer(&mut self, renderer_ref: &SharedMutRef<dyn Renderer>, gametime: f32) -> Result<()> {
        let mut renderer = renderer_ref.borrow_mut();

        let state = self.particle_state();

        match self.effect_type {
            RetailFireballEffectType::FadingLine => {
                let timelive = gametime - self.particle_state().creation_time;
                let size = self.particle_state().size;
                
                let norm_time = { 
                    let x = timelive / self.particle_state().life_time;
                    if x >= 1.0 {
                        // Don't go over!
                        0.99999
                    }
                    else {
                        x
                    }
                };

                renderer.set_alpha_type(AlphaType::SATURATE_VERTEX);
                renderer.set_texture_type(TextureType::Flat);
                renderer.set_lighting(LightStateType::Gouraud);
                renderer.set_color_model(ColorModelType::Rgb);
                renderer.set_overlay_type(OverlayTextureType::Blend);

                let mut vecs: [Vector; 2] = [
                    state.start_position,
                    state.end_position
                ];

                if !state.flags.contains(VisualEffectFlags::WINDSHIELD_EFFECT) {
                    let movement = state.movement_type.as_ref().unwrap();

                    match movement {
                        MovementType::Physical(physical) => {
                            let mut vel = physical.velocity;
                            Vector::normalize(&mut vel);
                            vecs[1] = state.start_position + (vel * state.size);
                        },
                        _ => return Err(anyhow!("VisualEffect required to use phyiscal movement type"))
                    }

                }

                let color = gr_16_to_color!(state.lighting_color);
                let (r, g, b) = (
                    gr_color_red!(color),
                    gr_color_green!(color),
                    gr_color_blue!(color)
                );

                for i in 0..2 {
                    
                }

            },
            _ => {}
        }

        todo!()
    }
}