use core::{cell::RefCell, sync::atomic::AtomicUsize};
use std::sync::Arc;

use bumpmap::BumpMap16;
use lightmap::LightMap16;
use rendering::Renderer;

use crate::common::SharedMutRef;

pub mod dd_video;
pub mod rendering;
pub mod bitmap;
pub mod bumpmap;
pub mod lightmap;
pub mod render_context;
pub mod drawing_2d;
pub mod polymodel;
pub mod texture;
pub mod procedural;
pub mod detail_settings;
pub mod generic_bitmap;
pub mod math;
pub mod drawing_3d;

use anyhow::Result;

pub type FrameCounter = Arc<AtomicUsize>;

/// bit depth info
#[macro_export]
macro_rules! bpp_to_bytespp {
    ($x:expr) => {
        (($x + 7) >> 3)
    };
}

pub enum BitsPerPixelType {
    // default for current display
    Default = 0,
    // 8-bit paletted.
    Bbp8 = 8,
    // 5-5-5 + chroma Hicolor
    Bbp15 = 15,
    // 5-6-5 Hicolor
    Bbp16 = 16,
    // 24 bit true color
    Bbp24 = 24,
    // 32 bit true color
    Bbp32 = 32,
}

pub const FIXED_SCREEN_WIDTH: usize = 640;
pub const FIXED_SCREEN_HEIGHT: usize = 480;

pub const OPAQUE_FLAG16: u16 = 0x8000;
pub const TRANSPARENT_COLOR32: u32 = 0x0000FF00;
pub const NEW_TRANSPARENT_COLOR: u32 = 0;
pub const OPAQUE_FLAG: u16 = OPAQUE_FLAG16;

pub const TEXTURE_WIDTH: usize = 128;
pub const TEXTURE_HEIGHT: usize = 128;
pub const TEXTURE_BPP: usize = 2;

pub type ddgr_color = u32;

pub const GR_NULL: ddgr_color =  0xFFFFFFFF;
pub const GR_BLACK: ddgr_color = 0x00000000;
pub const GR_GREEN: ddgr_color = 0x0000ff00;
pub const GR_RED: ddgr_color =   0x00ff0000;
pub const GR_BLUE: ddgr_color = 0x000000ff;
pub const GR_DARKGRAY: ddgr_color = 0x00404040;
pub const GR_LIGHTGRAY: ddgr_color = 0x00c0c0c0;
pub const GR_WHITE: ddgr_color = 0x00ffffff;

// ASCII 1 and (r,g,b) changes current text color in string.
pub const GR_COLOR_CHAR: u32 = 1;

pub enum MapSourceType16 {
    Bitmap(&SharedMutRef<dyn Bitmap16>),
    LightMap(&LightMap16),
    BumpMap(&BumpMap16),
}

#[derive(Debug, Clone, Copy)]
pub struct UVCoord {
    pub u: f32,
    pub v: f32
}

#[macro_export]
macro_rules! gr_rgb {
    ($r:expr, $g:expr, $b:expr) => {
        (($r << 16) | ($g << 8) | $b) as ddgr_color
    };
}

#[macro_export]
macro_rules! gr_rgb16 {
    ($r:expr, $g:expr, $b:expr) => {
        (($r as u16 >> 3) << 10) | (($g as u16 >> 3) << 5) | ($b as u16 >> 3)
    };
}

#[macro_export]
macro_rules! gr_color_to_16 {
    ($c:expr) => {{
        let r = (($c & 0x00FF0000) >> 16) as u16;
        let g = (($c & 0x0000FF00) >> 8) as u16;
        let b = ($c & 0x000000FF) as u16;
        
        (((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3)) as u16
    }};
}

#[macro_export]
macro_rules! gr_color_red {
    ($c:expr) => {
        (($c & 0x00FF0000) >> 16) as i32
    };
}

#[macro_export]
macro_rules! gr_color_green {
    ($c:expr) => {
        (($c & 0x0000FF00) >> 8) as i32
    };
}

#[macro_export]
macro_rules! gr_color_blue {
    ($c:expr) => {
        ($c & 0x000000FF) as i32
    };
}

#[macro_export]
macro_rules! gr_16_to_color {
    ($col:expr) => {
        gr_rgb!(
            (($col & 0x7C00) >> 7) * 255 / 31,  // Scale 5-bit red up to 8-bit
            (($col & 0x03E0) >> 2) * 255 / 31,  // Scale 5-bit green up to 8-bit
            (($col & 0x001F) << 3) * 255 / 31   // Scale 5-bit blue up to 8-bit
        )
    };
}

/// A trait for GPU resources that need to track their update status.
/// 
/// This trait provides methods to mark the resource as updated and to check
/// if the resource has been updated.
pub trait GpuMemoryResource {
    /// Marks the resource as updated.
    /// 
    /// This method should be called whenever the resource data is modified
    /// to indicate that the GPU needs to reload the resource from the CPU side.
    fn mark_updated(&mut self);

    /// Checks if the resource has been updated.
    /// 
    /// Returns `true` if the resource has been marked as updated, otherwise `false`.
    fn is_updated(&self) -> bool;
}

pub trait DrawableResource {
    fn draw_to_renderer(&mut self, renderer_ref: &SharedMutRef<dyn Renderer>, gametime: f32) -> Result<()>;
}

pub mod color_conversion {

    pub fn alpha_blend(src_color: u32, dst_color: u32) -> u32 {
        // Extract ARGB components from src_color
        let alpha_src = ((src_color >> 24) & 0xFF) as f32 / 255.0;
        let red_src = ((src_color >> 16) & 0xFF) as f32;
        let green_src = ((src_color >> 8) & 0xFF) as f32;
        let blue_src = (src_color & 0xFF) as f32;
    
        // Extract RGB components from dst_color (ignore alpha)
        let red_dst = ((dst_color >> 16) & 0xFF) as f32;
        let green_dst = ((dst_color >> 8) & 0xFF) as f32;
        let blue_dst = (dst_color & 0xFF) as f32;
    
        // Blend the color components
        let red_out = (alpha_src * red_src + (1.0 - alpha_src) * red_dst).min(255.0);
        let green_out = (alpha_src * green_src + (1.0 - alpha_src) * green_dst).min(255.0);
        let blue_out = (alpha_src * blue_src + (1.0 - alpha_src) * blue_dst).min(255.0);
    
        // Combine the components back into a 32-bit ARGB value with alpha set to 0
        let red_out_u8 = red_out as u32;
        let green_out_u8 = green_out as u32;
        let blue_out_u8 = blue_out as u32;
    
        (0x00 << 24) | (red_out_u8 << 16) | (green_out_u8 << 8) | blue_out_u8
    }
    
    pub fn additive_blend(color1: u32, color2: u32) -> u32 {
        // Extract ARGB components from color1
        let alpha1 = (color1 >> 24) & 0xFF;
        let red1 = (color1 >> 16) & 0xFF;
        let green1 = (color1 >> 8) & 0xFF;
        let blue1 = color1 & 0xFF;
    
        // Extract ARGB components from color2
        let alpha2 = (color2 >> 24) & 0xFF;
        let red2 = (color2 >> 16) & 0xFF;
        let green2 = (color2 >> 8) & 0xFF;
        let blue2 = color2 & 0xFF;
    
        // Add the components, ensuring they don't exceed 255
        let red = (red1 + red2).min(255);
        let green = (green1 + green2).min(255);
        let blue = (blue1 + blue2).min(255);
        let alpha = (alpha1 + alpha2).min(255); // Assuming additive blend for alpha as well
    
        // Combine the components back into a 32-bit ARGB value
        (alpha << 24) | (red << 16) | (green << 8) | blue
    }

    pub fn convert_4444_to_32(buffer: &[u16]) -> Vec<u32> {
        let mut buffer_32 = vec![0u32; buffer.len()];
        let mut i = 0;

        for &color in buffer {
            let a = ((color >> 12) & 0xF) as u32;
            let r = ((color >> 8) & 0xF) as u32;
            let g = ((color >> 4) & 0xF) as u32;
            let b = (color & 0xF) as u32;
    
            // Convert 4-bit colors to 8-bit colors
            let a = (a * 255 / 15) << 24;
            let r = (r * 255 / 15) << 16;
            let g = (g * 255 / 15) << 8;
            let b = b * 255 / 15;
    
            buffer_32[i] = a | r | g | b;
            i += 1;
        }

        buffer_32
    }

    pub fn convert_1555_to_32(buffer: &[u16]) -> Vec<u32> {
        let mut buffer_32 = vec![0u32; buffer.len()];
        let mut i = 0;

        for &color in buffer {
            // Extract individual components from the 15-bit ARGB1555 value
            let a = if (color & 0x8000) != 0 { 0xFFu32 } else { 0x0032 }; // Alpha: 1-bit
            let r = ((color as u32 >> 10) & 0x1F) * 255 / 31;              // Red: 5-bit
            let g = ((color as u32 >> 5) & 0x1F) * 255 / 31;               // Green: 5-bit
            let b = (color as u32 & 0x1F) * 255 / 31;                      // Blue: 5-bit
    
            buffer_32[i] = (a << 24) | (r << 16) | (g << 8) | b;
            i += 1;
        }

        buffer_32
    }

    pub fn convert_4444_to_grayscale(color: u16) -> u8 {
        // Extract 4-bit red, green, and blue channels
        let red = ((color >> 12) & 0xF) as u8;   // 4 bits
        let green = ((color >> 8) & 0xF) as u8;  // 4 bits
        let blue = ((color >> 4) & 0xF) as u8;   // 4 bits
    
        // Convert 4-bit values to 8-bit by scaling
        let red = (red * 255) / 15;
        let green = (green * 255) / 15;
        let blue = (blue * 255) / 15;
    
        // Calculate grayscale value
        let gray = 0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32;
        
        gray as u8
    }

    pub fn convert_1555_to_grayscale(color: u16) -> u8 {
        // Extract 5-bit red, green, and blue channels
        let red = ((color >> 10) & 0x1F) as u8;   // 5 bits for red
        let green = ((color >> 5) & 0x1F) as u8;  // 5 bits for green
        let blue = (color & 0x1F) as u8;          // 5 bits for blue
    
        // Convert 5-bit values to 8-bit by scaling from [0-31] to [0-255]
        let red = (red * 255) / 31;
        let green = (green * 255) / 31;
        let blue = (blue * 255) / 31;
    
        // Calculate grayscale value using luminance formula
        let gray = 0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32;
        
        gray as u8
    }

    pub fn convert_8_to_32(buffer: &[u8]) -> Vec<u32> {
        buffer.iter().map(|&gray| {
            let alpha = 0xFF; // Full alpha channel
            let grayscale = gray as u32; // Grayscale value used for R, G, and B
            (alpha << 24) | (grayscale << 16) | (grayscale << 8) | grayscale
        }).collect()
    }

    pub fn convert_16_to_32(buffer: &[u16]) -> Vec<u32> {
        buffer
            .iter()
            .map(|&gray| {
                let alpha = 0xFF; // Full alpha channel
                let grayscale = (gray >> 8) as u32; // Scale 16-bit to 8-bit by shifting
                (alpha << 24) | (grayscale << 16) | (grayscale << 8) | grayscale
            })
            .collect()
    }
}