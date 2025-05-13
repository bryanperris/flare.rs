use std::{env, fs::Metadata, path::{Path, PathBuf, MAIN_SEPARATOR}, sync::Once};
use env_logger::Env;
use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

use crate::graphics::color_conversion::{additive_blend, alpha_blend};


static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("trace"))
            .is_test(true) // Ensure it's suitable for test environment
            .init();
    });
}

fn generate_checkerboard(width: usize, height: usize, tile_size: usize) -> Vec<u32> {
    // Define the colors for the checkerboard
    let light_gray = 0xFFD3D3D3; // Light gray with full opacity
    let dark_gray = 0xFFA9A9A9;  // Dark gray with full opacity

    // Create a vector to hold the ARGB color values
    let mut checkerboard = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            // Determine the color of the current tile
            let is_light_gray_tile = ((x / tile_size) + (y / tile_size)) % 2 == 0;
            let color = if is_light_gray_tile { light_gray } else { dark_gray };

            // Add the color to the checkerboard array
            checkerboard.push(color);
        }
    }

    checkerboard
}

#[derive(Debug, Copy, Clone)]
pub enum BackgroundKind {
    Black,
    DarkGreen,
    Checkerboard
}

fn mix_with_background(buffer: &[u32], background: BackgroundKind, width: usize, height: usize) -> Vec<u32> {
    match background {
        BackgroundKind::DarkGreen => {
            buffer.into_iter()
            .enumerate()
            .map(|(i, color)| alpha_blend(color.to_owned(), 0xFF006400))
            .collect()
        },
        BackgroundKind::Checkerboard => {
            let checkerboard = generate_checkerboard(width, height, 20);
            buffer.into_iter()
            .enumerate()
            .map(|(i, color)| alpha_blend(color.to_owned(), checkerboard[i % checkerboard.len()]))
            .collect()
        }, 
        _ => {
            buffer.into_iter()
            .enumerate()
            .map(|(i, color)| alpha_blend(color.to_owned(), 0x00000000))
            .collect()
        }
    }

}

pub fn display_bitmap_4444(title: &str, buffer: &[u16], width: usize, height: usize, background: BackgroundKind) {
    use minifb::{Key, Window, WindowOptions};

    let mut window = Window::new(
        &format!("bitmap 4444: {} (Press ESC to close)", title),
        width,
        height,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {


        let mut colors = mix_with_background(
        &crate::graphics::color_conversion::convert_4444_to_32(&buffer),
        background, width, height);

        window
            .update_with_buffer(&colors, width, height)
            .unwrap();
    }
}

pub fn display_bitmap_argb32(title: &str, buffer: &[u32], width: usize, height: usize) {
    use minifb::{Key, Window, WindowOptions};

    let mut window = Window::new(
        &format!("bitmap argb32: {} (Press ESC to close)", title),
        width,
        height,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(&buffer, width, height)
            .unwrap();
    }
}

pub fn display_bitmap_1555(title: &str, buffer: &[u16], width: usize, height: usize) {
    use minifb::{Key, Window, WindowOptions};

    let mut window = Window::new(
        &format!("bitmap 1555: {} (Press ESC to close)", title),
        width,
        height,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(&crate::graphics::color_conversion::convert_1555_to_32(&buffer), width, height)
            .unwrap();
    }
}

#[macro_export]
macro_rules! display_4444 {
    ($title:expr, $buff:expr, $w:expr, $h:expr) => {
        #[cfg(feature = "bitmap_testview")]
        crate::test_common::display_bitmap_4444($title, $buff, $w, $h, crate::test_common::BackgroundKind::DarkGreen);
    };
}

#[macro_export]
macro_rules! display_4444_checkered {
    ($title:expr, $buff:expr, $w:expr, $h:expr) => {
        #[cfg(feature = "bitmap_testview")]
        crate::test_common::display_bitmap_4444($title, $buff, $w, $h, crate::test_common::BackgroundKind::Checkerboard);
    };
}

#[macro_export]
macro_rules! display_argb32 {
    ($title:expr, $buff:expr, $w:expr, $h:expr) => {
        #[cfg(feature = "bitmap_testview")]
        crate::test_common::display_bitmap_argb32($title, $buff, $w, $h);
    };
}


#[macro_export]
macro_rules! display_1555 {
    ($title:expr, $buff:expr, $w:expr, $h:expr) => {
        #[cfg(feature = "bitmap_testview")]
        crate::test_common::display_bitmap_1555($title, $buff, $w, $h);
    };
}


pub fn get_testdata_filepath(base_path: &str, filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env::current_dir().unwrap().to_str().unwrap());
    path.push(Path::new(base_path).parent().unwrap().to_str().unwrap());
    path.push(format!("{}{}{}", "testdata", MAIN_SEPARATOR, filename.to_owned()));

    path
}

#[macro_export]
macro_rules! testdata {
    ($filepath:expr) => {{
        crate::test_common::get_testdata_filepath(file!(), $filepath)
    }};
}

#[macro_export]
macro_rules! assert_md5 {
    ($data:expr, $hashstr: expr) => {
        let digest = md5::compute($data);
        let checksum = format!("{:x}", digest);
        assert_eq!(checksum, $hashstr);
    };
}