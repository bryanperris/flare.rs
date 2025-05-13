use prelude::*;
use room::Room;
use terrain::Terrain;

pub mod context;
pub mod prelude;
pub mod ambient_life;
pub mod object;
pub mod object_physics;
pub mod ai;
pub mod weapon;
pub mod object_static_behavior;
pub mod object_dynamic_behavior;
pub mod effects;
pub mod room;
pub mod geometry;
pub mod door;
pub mod scripting;
pub mod audio;
pub mod core;
pub mod node;
pub mod terrain;
pub mod weather;
pub mod physics;
pub mod visual_effects;

pub enum RegionRef {
    Room(SharedMutRef<Room>),
    Terrain((SharedMutRef<Terrain>, usize))
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct GameMode: u32 {
        /// Single player game.
        const SINGLE = 0b0000_0001;  // GM_SINGLE

        /// You are in network mode (multiplayer).
        const NETWORK = 0b0000_0100;  // GM_NETWORK

        /// You are in a modem (serial) game.
        const MODEM = 0b0010_0000;  // GM_MODEM

        /// The game has been finished (game over).
        const GAME_OVER = 0b1000_0000;  // GM_GAME_OVER

        /// Normal gameplay mode (no multiplayer).
        const NORMAL = 0b0000_0001;  // GM_NORMAL

        /// A combination of network and modem modes representing multiplayer.
        const MULTI = Self::NETWORK.bits() | Self::MODEM.bits();  // GM_MULTI
    }
}