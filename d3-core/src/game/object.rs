use super::{object_dynamic_behavior::DynBehaviorTable, prelude::*};

use core::{any::Any, cell::RefCell, marker::PhantomData, ops::Range};
use std::{collections::{HashMap, HashSet}, rc::{Rc, Weak}};
use crate::{graphics::lightmap::LightMap16, math::{matrix::Matrix, vector::Vector}, PAGENAME_LEN};

use super::object_static_behavior::BehaviorTable;

bitflags! {
    /// Object info flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct BehaviorFlags: u32 {
        const NONE = 0;
        /// This object uses AI
        const CONTROL_AI = 0x01;
        /// This object uses physics
        const USES_PHYSICS = 0x02;
        /// This object can be destroyed
        const DESTROYABLE = 0x04;
        /// This object can be selected in the inventory
        const INVEN_SELECTABLE = 0x08;
        /// This object can not be used by pressing ENTER during the game
        const INVEN_NONUSEABLE = 0x10;
        /// This object is for Mission objectives
        const INVEN_TYPE_MISSION = 0x20;
        /// This object should NOT be removed from the inventory when used
        const INVEN_NOREMOVE = 0x40;
        /// This object will not have its control type, movement type, and render types
        const INVEN_VISWHENUSED = 0x80;
        /// AI scripted death
        const AI_SCRIPTED_DEATH = 0x100;
        /// Do ceiling check
        const DO_CEILING_CHECK = 0x200;
        /// Ignore force fields and glass
        const IGNORE_FORCEFIELDS_AND_GLASS = 0x400;
        /// No difficulty scale damage
        const NO_DIFF_SCALE_DAMAGE = 0x800;
        /// No difficulty scale move
        const NO_DIFF_SCALE_MOVE = 0x1000;
        /// This object is just for show, & can be removed to improve performance
        const AMBIENT_OBJECT = 0x2000;
    }
}

#[derive(Debug, Clone)]
pub struct ObjectTypeDef {
    pub name: D3String,
    pub size: f32,
    pub flags: BehaviorFlags,
    pub score: i32,
    pub class: ObjectClass,
    pub behavior: BehaviorTable
}

#[derive(Debug, Clone, GameType)]
pub struct Object {
    typedef: ObjectTypeDef,
    pub dyn_behavior: DynBehaviorTable,

    pub name: D3String,
    pub control_type: (),
    pub render_type: (),
    pub lighting_type: (),

    pub room_num: Rc<()>,

    pub position: Vector,
    pub orientation: Matrix,
    pub last_position: Vector,

    pub renderframe: u16,

    pub wall_sphere_offset: Vector,
    pub anim_sphere_offset: Vector,

    pub size: f32,
    pub shields: f32,

    pub contains: HashMap<ObjectClass, Rc<Object>>,
    
    pub creation_time: f32,
    pub lifeleft: f32,
    pub lifetime: f32,

    // TODO: Attachment stuff
    pub link_prev_obj: Option<SharedMutRef<Object>>,
    pub link_next_obj: Option<SharedMutRef<Object>>,

    pub weapon_fire_flags: (),

    // Collision detection stuff
    pub min_xzy: Vector,
    pub max_xzy: Vector,

    // Object change info
    pub change_flags: i32,

    // generic visualization flags
    pub generic_nonvis_flags: i32,
    pub generic_sent_nonvis: i32,

    pub lightmap: LightMap16,

    // Nice Comment:
    // Something to do with multiplayer, possibly, but it's hard to know for sure
    // because some people are incapable of commented their code.
    pub position_counter: u16,

    pub parent_room: Weak<RefCell<super::room::Room>>
}

impl Object {
    pub fn new() -> Self { todo!() }
    pub fn typedef(&self) -> &ObjectTypeDef {
        &self.typedef
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectClass {
    /// A wall... not really an object, but used for collisions.
    Wall,
    /// A fireball, part of an explosion.
    Fireball,
    /// An evil enemy.
    Robot,
    /// A piece of glass.
    Shard,
    /// The player on the console.
    Player,
    /// A laser, missile, etc.
    Weapon,
    /// A viewed object in the editor.
    Viewer,
    /// A powerup you can pick up.
    Powerup,
    /// A piece of robot.
    Debris,
    /// A camera object in the game.
    Camera,
    /// A shockwave.
    Shockwave,
    /// Misc objects.
    Clutter,
    /// What the player turns into when dead.
    Ghost,
    /// A light source, & not much else.
    Light,
    /// A cooperative player object.
    Coop,
    /// A map marker.
    Marker,
    /// A building.
    Building,
    /// A door.
    Door,
    /// A room, visible on the terrain.
    Room,
    /// A particle.
    Particle,
    /// A splinter piece from an exploding object.
    Splinter,
    /// A dummy object, ignored by everything.
    Dummy,
    /// An observer in a multiplayer game.
    Observer,
    /// Something for debugging.
    DebugLine,
    /// An object that makes a sound but does nothing else.
    SoundSource,
    /// An object that marks a waypoint.
    Waypoint,
}

impl From<usize> for ObjectClass {
    fn from(value: usize) -> Self {
        match value {
            0 => ObjectClass::Wall,
            1 => ObjectClass::Fireball,
            2 => ObjectClass::Robot,
            3 => ObjectClass::Shard,
            4 => ObjectClass::Player,
            5 => ObjectClass::Weapon,
            6 => ObjectClass::Viewer,
            7 => ObjectClass::Powerup,
            8 => ObjectClass::Debris,
            9 => ObjectClass::Camera,
            10 => ObjectClass::Shockwave,
            11 => ObjectClass::Clutter,
            12 => ObjectClass::Ghost,
            13 => ObjectClass::Light,
            14 => ObjectClass::Coop,
            15 => ObjectClass::Marker,
            16 => ObjectClass::Building,
            17 => ObjectClass::Door,
            18 => ObjectClass::Room,
            19 => ObjectClass::Particle,
            20 => ObjectClass::Splinter,
            21 => ObjectClass::Dummy,
            22 => ObjectClass::Observer,
            23 => ObjectClass::DebugLine,
            24 => ObjectClass::SoundSource,
            25 => ObjectClass::Waypoint,
            _ => panic!("Invalid ObjectClass value: {}", value),
        }
    }
}


