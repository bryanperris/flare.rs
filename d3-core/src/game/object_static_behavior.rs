use core::{any::Any, ops::Range};
use std::rc::Rc;

use bitflags::bitflags;
use paste::paste;

use crate::math::vector::Vector;

use super::weapon::{
    DynamicWeaponBatteryFlags, StaticWeaponBatteryFlags, MAX_FIRING_MASKS, MAX_GUNPOINTS,
    MAX_TURRETS, MAX_UPGRADES,
};

// #[macro_export]
// macro_rules! define_object_behavior {
//     // Match for non-generic structs with optional documentation comments
//     ($struct_name:ident { $($(#[$meta:meta])* $field_name:ident: $field_type:ty),* $(,)? }) => {
//         paste::paste! {
//             #[derive(Debug, Clone, PartialEq)]
//             pub struct [<$struct_name BehaviorInfo>] {
//                 $(
//                     $(#[$meta])*
//                     pub $field_name: $field_type,
//                 )*
//             }

//             // pub trait [<$struct_name BehaviorType>] {
//             //     fn as_self(&self) -> &[<$struct_name BehaviorInfo>];
//             // }

//             // impl [<$struct_name BehaviorType>] for [<$struct_name BehaviorInfo>] {
//             //     fn as_self(&self) -> &Self {
//             //         self
//             //     }
//             // }
//         }
//     };
//     // Match for generic structs with optional documentation comments and trait bounds
//     ($struct_name:ident < $($gen:ident : $bound:ident),+ > { $($(#[$meta:meta])* $field_name:ident: $field_type:ty),* $(,)? }) => {
//         paste::paste! {
//             #[derive(Debug, Clone, PartialEq)]
//             pub struct [<$struct_name BehaviorInfo>] < $($gen: $bound),+ > {
//                 $(
//                     $(#[$meta])*
//                     pub $field_name: $field_type,
//                 )*
//             }
//             // pub trait [<$struct_name BehaviorType>] < $($gen: $bound),+ > {
//             //     fn as_self(&self) -> &[<$struct_name BehaviorInfo>] < $($gen),+ >;
//             // }

//             // impl< $($gen: $bound),+ > [<$struct_name BehaviorType>] < $($gen),+ > for [<$struct_name BehaviorInfo>] < $($gen),+ > {
//             //     fn as_self(&self) -> &Self {
//             //         self
//             //     }
//             // }
//         }
//     };
// }

#[derive(Debug, Clone)]
pub struct BehaviorTable {
    pub drawable: Option<Drawable<Rc<dyn Any>>>,
    pub light: Option<Light>,
    pub destroyable: Option<Destroyable>,
    pub powerup: Option<Powerup>,
    pub inventory: Option<Inventory>,
    pub animated: Option<Animated>,
    pub scripted: Option<Scripted>,
    pub multiplayer: Option<Multiplayer>,
    pub drawable_weapon_battery: Option<DrawableWeaponBattery>,
    pub static_weapon_battery: Option<StaticWeaponBattery>,
    pub physical: Option<Physical>,
    pub autonomous: Option<Autonomous>,
}

#[derive(Debug, Clone)]
pub struct Drawable<T> {
    pub resolution_h: Rc<T>,
    pub resolution_m: Rc<T>,
    pub resolution_l: Rc<T>,
    pub lod_distance_m: f32,
    pub lod_distance_l: f32,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub flags: i32,
    pub light_distance: f32,
    pub red_light1: f32,
    pub green_light1: f32,
    pub blue_light1: f32,
    pub red_light2: f32,
    pub green_light2: f32,
    pub blue_light2: f32,
    pub time_interval: f32,
    pub flicker_distance: f32,
    pub directional_dot: f32,
    pub timebits: i32,
    pub angle: u8,
    pub lighting_render_type: u8,
}

#[derive(Debug, Clone)]
pub struct Destroyable {
    pub hit_points: i32,
    pub damage: f32,
    pub impact_size: f32,
    pub impage_time: f32,
}

#[derive(Debug, Clone)]
pub struct Powerup {
    pub ammo: i32,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub description: String,
    pub icon_name: String,
}

#[derive(Debug, Clone)]
pub struct AnimationEntry {
    range: Range<u16>,
    spc: f32,
    sound_index: usize, // TODO
}

#[derive(Debug, Clone)]
pub struct Multiplayer {
    pub respawn: f32,
}

#[derive(Debug, Clone)]
pub struct Animated {
    entries: Box<[AnimationEntry]>,
}

#[derive(Debug, Clone)]
pub struct Scripted {
    name: String,
    name_override: String, // fn module_name(&self) -> &[u8; 32];
                           // fn script_name_override(&self) -> &[u8; PAGENAME_LEN];
}

// TODO: We rather store RC based references to polymodels
#[derive(Debug, Clone)]
pub struct DrawableWeaponBattery {
    pub num_gunpoints: usize,
    pub gunpoint_index: [usize; MAX_GUNPOINTS],
    pub num_turrets: usize,
    pub turrent_index: [usize; MAX_TURRETS],
}

#[derive(Debug, Clone)]
pub struct StaticWeaponBattery {
    pub gp_weapon_index: [u16; MAX_GUNPOINTS],
    pub fm_fire_sound_index: [u16; MAX_FIRING_MASKS],
    pub aiming_gp_index: u16,

    pub num_masks: usize,
    pub gp_fire_masks: [u8; MAX_FIRING_MASKS],
    pub gp_fire_wait: [f32; MAX_FIRING_MASKS],

    pub gp_quad_fire_mask: u8,

    pub num_levels: usize,
    pub gp_level_weapon_index: [u16; MAX_UPGRADES],
    pub gp_level_fire_sound_index: [u16; MAX_UPGRADES],

    pub aiming_flags: u8,
    pub aiming_3d_dot: f32,
    pub aiming_3d_dist: f32,
    pub aiming_xz_dot: f32,

    pub anim_start_frame: [f32; MAX_FIRING_MASKS],
    pub anim_fire_frame: [f32; MAX_FIRING_MASKS],
    pub anim_end_frame: [f32; MAX_FIRING_MASKS],
    pub anim_time: [f32; MAX_FIRING_MASKS],

    pub flags: StaticWeaponBatteryFlags,

    pub energy_usage: f32,
    pub ammo_usage: f32,
}

/// Represents either a rotational velocity vector or a turn rate.
#[derive(Debug, Clone, Copy, PartialEq)]
enum RotVelOrTurnRate {
    /// Rotational velocity (angles).
    RotVel(Vector),
    /// Turn rate.
    TurnRate(f32),
}

/// Represents either a full thrust magnitude or a maximum velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FullThrustOrMaxVelocity {
    /// Maximum thrust magnitude.
    FullThrust(f32),
    /// Maximum velocity.
    MaxVelocity(f32),
}

/// Represents either a full rotational thrust magnitude or a maximum turn rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FullRotThrustOrMaxTurnRate {
    /// Maximum rotation thrust magnitude.
    FullRotThrust(f32),
    /// Maximum turn rate.
    MaxTurnRate(f32),
}

/// Represents either a hit die dot or a stuck room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitDieDotOrStuckRoom {
    /// Hit die dot.
    HitDieDot(f32),
    /// Stuck room.
    StuckRoom(i32),
}

/// Represents either a maximum speed time or a stuck portal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaxSpeedTimeOrStuckPortal {
    /// Maximum speed time.
    MaxSpeedTime(f32),
    /// Stuck portal.
    StuckPortal(i32),
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct PhysicsFlags: u32 {
        const NONE = 0;
        /// Roll when turning.
        const TURNROLL = 0x01;  // PF_TURNROLL

        /// Level object with closest side.
        const LEVELING = 0x02;  // PF_LEVELING

        /// Bounce (not slide) when hit.
        const BOUNCE = 0x04;  // PF_BOUNCE

        /// Wiggle while flying.
        const WIGGLE = 0x08;  // PF_WIGGLE

        /// Object sticks (stops moving) when hitting a wall.
        const STICK = 0x10;  // PF_STICK

        /// Object keeps going even after hitting another object (e.g., fusion cannon).
        const PERSISTENT = 0x20;  // PF_PERSISTENT

        /// This object uses its thrust.
        const USES_THRUST = 0x40;  // PF_USES_THRUST

        /// Affected by gravity.
        const GRAVITY = 0x80;  // PF_GRAVITY

        /// Affected by magnetism (ignores its own concentric forces).
        const IGNORE_OWN_CONC_FORCES = 0x100;  // PF_IGNORE_OWN_CONC_FORCES

        /// Affected by wind.
        const WIND = 0x200;  // PF_WIND

        /// Uses the velocity of its parent.
        const USES_PARENT_VELOCITY = 0x400;  // PF_USES_PARENT_VELOCITY

        /// Has a fixed velocity.
        const FIXED_VELOCITY = 0x800;  // PF_FIXED_VELOCITY

        /// Has a fixed rotational velocity.
        const FIXED_ROT_VELOCITY = 0x1000;  // PF_FIXED_ROT_VELOCITY

        /// Cannot collide with its parent.
        const NO_COLLIDE_PARENT = 0x2000;  // PF_NO_COLLIDE_PARENT

        /// Can collide with its siblings (e.g., bombs).
        const HITS_SIBLINGS = 0x4000;  // PF_HITS_SIBLINGS

        /// Flies upward with gravity (reverse gravity).
        const REVERSE_GRAVITY = 0x8000;  // PF_REVERSE_GRAVITY

        /// No collisions and no relinks (dangerous if not used correctly).
        const NO_COLLIDE = 0x10000;  // PF_NO_COLLIDE

        /// No collisions with robots.
        const NO_ROBOT_COLLISIONS = 0x20000;  // PF_NO_ROBOT_COLLISIONS

        /// Point collisions with walls (radius set to zero when colliding with walls).
        const POINT_COLLIDE_WALLS = 0x40000;  // PF_POINT_COLLIDE_WALLS

        /// The object (weapon) homes in on targets.
        const HOMING = 0x80000;  // PF_HOMING

        /// The object is guided (e.g., missile).
        const GUIDED = 0x100000;  // PF_GUIDED

        /// Ignores concussive forces.
        const IGNORE_CONCUSSIVE_FORCES = 0x200000;  // PF_IGNORE_CONCUSSIVE_FORCES

        /// Has a destination position.
        const DESTINATION_POS = 0x400000;  // PF_DESTINATION_POS

        /// Locks the object's movement in the X axis.
        const LOCK_X = 0x800000;  // PF_LOCK_X

        /// Locks the object's movement in the Y axis.
        const LOCK_Y = 0x1000000;  // PF_LOCK_Y

        /// Locks the object's movement in the Z axis.
        const LOCK_Z = 0x2000000;  // PF_LOCK_Z

        /// Locks the object's pitch (P axis).
        const LOCK_P = 0x4000000;  // PF_LOCK_P

        /// Locks the object's heading (H axis).
        const LOCK_H = 0x8000000;  // PF_LOCK_H

        /// Locks the object's bank (B axis).
        const LOCK_B = 0x10000000;  // PF_LOCK_B

        /// This object should never use a big collision sphere.
        const NEVER_USE_BIG_SPHERE = 0x20000000;  // PF_NEVER_USE_BIG_SPHERE

        /// This object doesn't collide with objects of the same type.
        const NO_SAME_COLLISIONS = 0x40000000;  // PF_NO_SAME_COLLISIONS

        /// No collisions occur with doors.
        const NO_DOOR_COLLISIONS = 0x80000000;  // PF_NO_DOOR_COLLISIONS

        /// A combination of regular gravity and reverse gravity.
        const GRAVITY_MASK = Self::GRAVITY.bits() | Self::REVERSE_GRAVITY.bits();  // PF_GRAVITY_MASK

        /// A combination of all lock-related flags (movement and rotation locks).
        const LOCK_MASK = Self::LOCK_X.bits() | Self::LOCK_Y.bits() | Self::LOCK_Z.bits() |
                          Self::LOCK_P.bits() | Self::LOCK_H.bits() | Self::LOCK_B.bits();  // PF_LOCK_MASK
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Physical {
    /// Velocity vector of this object.
    pub velocity: Vector,
    /// Constant force applied to this object.
    pub thrust: Vector,
    /// Rotational velocity or turn rate.
    pub rot_vel_or_turn_rate: Option<RotVelOrTurnRate>,
    /// Rotational acceleration.
    pub rot_thrust: Vector,
    /// Rotation caused by turn banking.
    pub turn_roll: f32,
    /// The current delta position a wiggle has caused.
    pub last_still_time: f32,
    /// Number of bounces before exploding.
    pub num_bounces: i32,
    /// Percent of velocity kept after a bounce.
    pub coeff_restitution: f32,
    /// Mass of this object.
    pub mass: f32,
    /// How fast this object slows down.
    pub drag: f32,
    /// Resistance to change in spin rate.
    pub rot_drag: f32,
    /// Full thrust magnitude or maximum velocity.
    pub full_thrust_or_max_velocity: Option<FullThrustOrMaxVelocity>,
    /// Full rotational thrust magnitude or maximum turn rate.
    pub full_rot_thrust_or_max_turn_rate: Option<FullRotThrustOrMaxTurnRate>,
    /// Maximum turn roll rate.
    pub max_turn_roll_rate: f32,
    /// Roll for a given turning rate.
    pub turn_roll_ratio: f32,
    /// Amplitude of an object's wiggle.
    pub wiggle_amplitude: f32,
    /// Speed of wiggle.
    pub wiggles_per_sec: f32,
    /// Destination position for interpolating velocity (for multiplayer only).
    pub dest_pos: Vector,
    /// Hit die dot or stuck room.
    pub hit_die_dot_or_stuck_room: Option<HitDieDotOrStuckRoom>,
    /// Maximum speed time or stuck portal.
    pub max_speed_time_or_stuck_portal: Option<MaxSpeedTimeOrStuckPortal>,
    /// Miscellaneous physics flags.
    pub flags: PhysicsFlags,
}

impl Default for Physical {
    fn default() -> Self {
        Self {
            velocity: Default::default(),
            thrust: Default::default(),
            rot_vel_or_turn_rate: None,
            rot_thrust: Default::default(),
            turn_roll: Default::default(),
            last_still_time: Default::default(),
            num_bounces: Default::default(),
            coeff_restitution: Default::default(),
            mass: Default::default(),
            drag: Default::default(),
            rot_drag: Default::default(),
            full_thrust_or_max_velocity: Default::default(),
            full_rot_thrust_or_max_turn_rate: Default::default(),
            max_turn_roll_rate: Default::default(),
            turn_roll_ratio: Default::default(),
            wiggle_amplitude: Default::default(),
            wiggles_per_sec: Default::default(),
            dest_pos: Default::default(),
            hit_die_dot_or_stuck_room: Default::default(),
            max_speed_time_or_stuck_portal: Default::default(),
            flags: PhysicsFlags::NONE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Autonomous {
    pub ai_class: char,
    pub ai_type: char,
    pub max_velocity: f32,
    pub max_delta_velocity: f32,
    pub max_turn_rate: f32,
    pub max_delta_turn_rate: f32,
    pub attack_vel_percent: f32,
    pub flee_vel_percent: f32,
    pub dodge_vel_percent: f32,
    pub circle_distance: f32,
    pub dodge_percent: f32,
    pub melee_damage: [f32; 2],
    pub melee_latency: [f32; 2],
    pub sound: Vec<i32>,
    pub movement_type: char,
    pub movement_subtype: char,
    pub flags: i32,
    pub notify_flags: i32,
    pub fov: f32,
    pub avoid_friends_distance: f32,
    pub frustration: f32,
    pub curiousity: f32,
    pub life_preservation: f32,
    pub agression: f32,
    pub fire_spread: f32,
    pub night_vision: f32,
    pub fog_vision: f32,
    pub lead_accuracy: f32,
    pub lead_varience: f32,
    pub fight_team: f32,
    pub fight_same: f32,
    pub hearing: f32,
    pub roaming: f32,
    pub biased_flight_importance: f32,
    pub biased_flight_min: f32,
    pub biased_flight_max: f32,
}
