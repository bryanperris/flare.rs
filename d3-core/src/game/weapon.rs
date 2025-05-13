use bitflags::bitflags;

use crate::math::vector::Vector;

pub const MAX_GUNPOINTS: usize = 8;
pub const MAX_FIRING_MASKS: usize = 8;
pub const MAX_TURRETS: usize = 8;
pub const MAX_UPGRADES: usize = 5;
pub const MAX_WEAPON_BATTERIES_PER_OBJECT: usize = 21;

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct StaticWeaponBatteryFlags: u16 {
        const SPRAY = 1;
        const ANIM_LOCAL = 2;
        const ANIM_FULL = 4;
        const ANIM_MASKS = 6;
        const RANDOM_FIRE_ORDER = 8;
        const GUIDED = 16;
        const USE_CUSTOM_FOV = 32;
        const ON_OFF = 64;
        const USE_CUSTOM_MAX_DIST = 128;
        const USER_TIMEOUT = 256;
        const FIRE_FVEC = 512;
        const AIM_FVEC = 1024;
        const FIRE_TARGET = 2048;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct DynamicWeaponBatteryFlags: u8 {
        const ENABLED = 1;
        const AUTOMATIC = 2;
        const ANIMATING = 4;
        const ANIM_FIRED = 8;
        const QUAD = 16;
        const UPGRADED = 32;
    }
}