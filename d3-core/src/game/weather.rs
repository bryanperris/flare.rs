use super::prelude::*;

const MAX_RAIN_INTENSITY: f32 = 50.0;
const MAX_SNOW_INTENSITY: f32 = 200.0;

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct WeatherFlags: u32 {
        const NONE = 0;
        const RAIN = 1;
        const LIGHTNING = 2;
        const SNOW = 4;
    }
}

#[derive(Debug, Clone, GameType)]
pub struct Weather {
    pub flags: WeatherFlags,
    pub snow_intensity_scalar: f32,
    pub rain_intensity_scalar: f32,
    pub rain_color: i32,
    pub lightning_color: i32,
    pub sky_flash_color: i32,

    pub lighting_sequence: u8,
    pub last_lighting_evaluation_time: f32,
    pub lighting_interval_time: f32,
    pub lightning_rand_value: i32,

    pub snowflakes_to_create: usize,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            flags: WeatherFlags::NONE,
            ..Default::default()
        }
    }
}

impl Weather {
    
}

impl GameBoundedType<Weather> {

}
