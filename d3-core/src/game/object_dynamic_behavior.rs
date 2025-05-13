use std::rc::Rc;
use crate::{graphics::{bitmap::Bitmap16, ddgr_color}, math::vector::Vector};

use super::{effects::*, object::Object, object_static_behavior::{Autonomous, Light, Physical}, weapon::{DynamicWeaponBatteryFlags, MAX_TURRETS}};

#[derive(Debug, Clone)]
pub struct DynBehaviorTable {
    pub movement: Option<MovementType>,
    pub weapon_battery: Option<DynamicWeaponBattery>,
    pub control: Option<ControlType>,
    pub autonomous: Option<Autonomous>,
    pub shockwave: Option<ShockwaveEmitter>,
    pub explosive: Option<Explosive>,
    pub laser: Option<LaserEmitter>,
    pub powerup: Option<Powerup>,
    pub splinter: Option<Splinter>,
    pub blast: Option<BlastEmitter>,
    pub dying: Option<DeathEmitter>,
    pub debris: Option<Debris>,
    pub audible: Option<SoundEmitter>,
    pub drawable: Option<DrawableType>,
    pub effects: Option<EffectEmitter>,
    pub scripting: Option<ScriptedRuntime>
}

#[derive(Debug, Clone)]
pub struct ShockwaveEmitter {
    pub damaged: Vec<Rc<Object>>
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub parent: Rc<Object>,
    pub forward: Vector,
    pub up: Vector,
    pub position: Vector
}

#[derive(Debug, Clone)]
pub enum MovementType {
    Physical(Physical),
    Shockwave(ShockwaveEmitter),
    Attachment(Attachment),
    Walking,
    AtRest,
}

#[derive(Debug, Clone)]
pub struct DynamicWeaponBattery {
    pub last_fire_time: f32,
    pub cur_firing_mask: u8,

    pub norm_turret_angle: [f32; MAX_TURRETS],
    pub turret_next_think_time: [f32; MAX_TURRETS],
    pub turret_direction: [u8; MAX_TURRETS],

    pub wb_anim_mask: u8,
    pub wb_anim_frame: f32,

    pub cur_target: Vector,

    pub upgrade_level: i8, // Assuming char is signed
    pub flags: DynamicWeaponBatteryFlags,
}

#[derive(Debug, Clone)]
pub struct Explosive {
    pub impact_size: f32,
    pub impact_time: f32,
    pub impact_player_damage: f32,
    pub impact_generic_damage: f32,
    pub impact_force: f32
}

#[derive(Debug, Clone)]
pub struct LaserEmitter {
    pub parent: Rc<Object>,
    pub src_gunpoint: Rc<()>,

    /// For persistent weapons (survive object collision), object it most recently hit.
    pub last_hit_handle: (),
    pub tracking: Option<Rc<Object>>,

    /// Last track time (see if an object is visible)
    pub last_track_time: f32,

    pub hit_status: (),
    pub hit_point: Vector,
    pub hit_wall_point: Vector,
    pub hit_wall_normal: Vector,
    pub hit_room: (),
    pub hit_point_room: (),
    pub hit_face: i16,

    ///	Power if this is a fusion bolt (or other super weapon to be added).
    pub multiplier: f32,

    /// How many seconds of thrust are left before the weapon stops thrusting
    pub thrust_left: f32,

    // Last time a particle was dropped from this weapon
    pub last_drop_time: f32,
    // Last place smoke was dropped from this weapon
    pub last_smoke_position: Vector,
    // Whether or not this weapon casts light
    pub does_cast_light: bool
}

#[derive(Debug, Clone)]
pub struct Powerup {
    /// how many/much we pick up (vulcan cannon only?)
    count: i32
}

#[derive(Debug, Clone)]
pub struct Splinter {
    pub child: Rc<Object>,
    pub facenum: i16,
    pub verticies: Vec<Vector>,
    pub center: Vector
}

#[derive(Debug, Clone)]
pub struct BlastEmitter {
    pub max_size: f32,
    pub bitmap: Rc<dyn Bitmap16>
}

#[derive(Debug, Clone)]
pub struct DeathEmitter {
    pub flags: (),
    /// How long until object dies
    pub delay_time: f32,
    /// The player who wille this object, or -1 if not a player
    pub killer_player: Option<Rc<Object>>,

    pub last_spark_time: f32,
    pub last_fireball_time: f32,
    pub last_smoke_time: f32
}

#[derive(Debug, Clone)]
pub struct Debris {
    pub death_flags: (), // copy of parents
    pub last_smoke_time: f32
}

#[derive(Debug, Clone)]
pub struct SoundEmitter {
    pub sound_index: (),
    pub volume: f32
}

#[derive(Debug, Clone)]
pub enum ControlType {
    Laser(LaserEmitter),
    Powerup(Powerup),
    Splinter(Splinter),
    Blast(BlastEmitter),
    Dying(DeathEmitter),
    Debris(Debris),
    SoundSource(SoundEmitter)
}

#[derive(Debug, Clone)]
pub struct MultiTurrent {
    pub time: f32,
    pub last_time: f32,
    pub num: usize,
    pub last_keyframe: (),
    pub keyframes: Vec<()>
}

#[derive(Debug, Clone)]
pub struct CustomAnimation {
    pub server_time: f32,
    pub server_animation_frame: u16,
    pub animation_start_frame: u16,
    pub animation_end_frame: u16,
    pub animation_time: f32,
    pub max_speed: f32,
    pub animation_sound: (),
    pub flags: (),
    pub next_animation_type: ()
}

#[derive(Debug, Clone)]
pub enum CustomAnimationType {
    Client(CustomAnimation),
    Server(CustomAnimation)
}

#[derive(Debug, Clone)]
pub struct DrawablePolyModel {
    pub model: Rc<()>,
    pub dying_model: Rc<()>,
    pub animation_start_frame: f32,
    pub animation_frame: f32,
    pub animation_end_frame: f32,
    pub animation_time: f32,
    pub animation_flags: (),
    pub max_speed: f32,
    pub animation: CustomAnimationType,
    pub multi_turret: MultiTurrent,
    pub child_flags: (),
    pub tmap_override: Option<()>
}

#[derive(Debug, Clone)]
pub struct Shard {
    points: [Vector; 3],
    u: [f32; 3],
    v: [f32; 3],
    normal: Vector,
    tmap: ()
}

#[derive(Debug, Clone)]
pub enum DrawableType {
    Polymodel(DrawablePolyModel),
    Shard(Shard),
    SphereColor(ddgr_color)
}

#[derive(Debug, Clone)]
pub struct EffectEmitter {
    pub is_napalmed: bool,
    pub is_negative_light: bool,
    pub is_virus_infected: bool,
    pub is_bumpmapped: bool,
    pub color: Option<ColoredEffect>,
    pub cloak: Option<CloakEffect>,
    pub deform: Option<DeformEffect>,
    pub damage: Option<DamageEffect>,
    pub fade: Option<FadeEffectType>,
    pub audio: Option<AudibleEffect>,
    pub powerup: Option<PowerupEffect>,
    pub light: Option<Light>,
    pub spec_light: Option<SpecularLightEffect>,
    pub dyn_light: Option<DynamicVolumeLightEffect>,
    pub liquid: Option<LiquidEffect>,
    pub freeze: Option<FreezeEffect>,
    pub grapple: Option<AttachmentEffect>,
    pub spark: Option<SparkEffect>
}

#[derive(Debug, Clone)]
pub struct ScriptedRuntime {
    // TODO:
}