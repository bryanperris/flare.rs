#![cfg_attr(not(feature = "std"), no_std)]

// TODO: XXX: DISABLE ALL WARNINGS FOR NOW!!!!
// TODO: REMOVE THIS EVENTUALLY!
#![allow(warnings)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate gametype_macro;

extern crate blake3;

pub mod common;

pub mod graphics;

pub mod osirus;
pub mod game_client;
pub mod endianess;
pub mod filesystem;
pub mod string_common;
pub mod retail;
pub mod game;
pub mod math;
pub mod string;
pub mod rand;


#[cfg(test)]
pub mod test_common;

pub fn get_version() -> &'static str {
    return "TODO: get version";
}

pub fn create_rng() -> impl tinyrand::Rand {
    extern crate tinyrand;

    #[cfg(feature = "std")]
    extern crate tinyrand_std;
    
    use tinyrand::{Rand, StdRand, Seeded};
    
    #[cfg(feature = "std")]
    use tinyrand_std::clock_seed::ClockSeed;

    #[cfg(feature = "std")]
    {
        let seed = ClockSeed::default().next_u64();
        StdRand::seed(seed)
    }

    #[cfg(not(feature = "std"))]
    {
        StdRand::default()
    }
}

pub const PAGENAME_LEN: usize = 35;