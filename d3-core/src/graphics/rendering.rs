use bitflags::bitflags;

use super::{ddgr_color, drawing_2d::font::FontGlyph};
use crate::graphics::drawing_2d::font::FontGraphic;

bitflags! {
    pub struct AlphaTypeFlags: i8 {
        /// Take constant alpha into account
        const Constant = 0b0001;
        /// Take texture alpha into account
        const Texture =  0b0010;
        /// Take vertex alpha into account
        const Vertex =   0b0100;
    }
}

pub enum TextureType {
    /// Solid Color
    Flat,
    /// Textured Linearly
    Linear,
    /// Textured Perspectively
    Perspective,
    /// A textured polygon drawn as a flat color
    LinearSpecial,
    /// A textured polygon drawn as a flat color
    PerspectiveSpecial
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AlphaType: u32 {
        /// Alpha is always fully opaque (255 or 1.0).
        const ALWAYS = 1 << 0;

        /// Uses a constant alpha value across the entire image.
        const CONSTANT = 1 << 1;

        /// Uses only the alpha channel from the texture.
        const TEXTURE = 1 << 2;

        /// Multiplies the texture alpha with a constant alpha value.
        const CONSTANT_TEXTURE = 1 << 3;

        /// Uses only the vertex alpha values.
        const VERTEX = 1 << 4;

        /// Multiplies the vertex alpha with a constant alpha value.
        const CONSTANT_VERTEX = 1 << 5;

        /// Multiplies the texture alpha with the vertex alpha.
        const TEXTURE_VERTEX = 1 << 6;

        /// Multiplies texture alpha, vertex alpha, and a constant alpha.
        const CONSTANT_TEXTURE_VERTEX = 1 << 7;

        /// Blends destination and source colors.
        const LIGHTMAP_BLEND = 1 << 8;

        /// Saturates the blending effect up to white.
        const SATURATE_TEXTURE = 1 << 9;

        /// Similar to `LIGHTMAP_BLEND`, but applied to Gouraud-shaded flat polygons.
        const FLAT_BLEND = 1 << 10;

        /// Draws an anti-aliased polygon.
        const ANTIALIAS = 1 << 11;

        /// Applies saturation using vertex alpha.
        const SATURATE_VERTEX = 1 << 12;

        /// Multiplies constant alpha with vertex alpha for saturation.
        const SATURATE_CONSTANT_VERTEX = 1 << 13;

        /// Multiplies texture alpha with vertex alpha for saturation.
        const SATURATE_TEXTURE_VERTEX = 1 << 14;

        /// Like `LIGHTMAP_BLEND`, but also considers vertex alpha.
        const LIGHTMAP_BLEND_VERTEX = 1 << 15;

        /// Like `LIGHTMAP_BLEND`, but also considers constant alpha.
        const LIGHTMAP_BLEND_CONSTANT = 1 << 16;

        /// Specular lighting effect.
        const SPECULAR = 1 << 17;

        /// Like `LIGHTMAP_BLEND`, but performs addition instead of multiplication.
        const LIGHTMAP_BLEND_SATURATE = 1 << 18;
    }
}

pub enum OverlayTextureType {
    /// No overlay
    None,
    /// Draw a lightmap texture afterwards
    Blend,
    /// Draw a tmap2 style texture afterwards
    Replace,
    /// Draw a gouraud shaded polygon afterwards
    FlatBlend,
    /// Like OT_BLEND, but take constant alpha into account
    BlendVertex,
    /// Draw a saturated bumpmap afterwards
    Bumpmap,
    /// Add a lightmap in
    BlendSaturate
}

pub enum LightStateType {
    /// No lighting, fully lit
    None,
    Gouraud,
    Phong,
    /// Take color from flat color
    FlatGouraud
}

pub enum ColorModelType {
    /// monochromatic (intensity) model - default
    Mono,
    Rgb
}

pub trait Renderer {
    fn set_flat_color(&mut self, color: ddgr_color);

    fn draw_font_char(&mut self, font_graphic: &FontGraphic, glyph: &FontGlyph);
    
    fn set_texture_type(&mut self, texture_type: TextureType);

    fn set_overlay_type(&mut self, overlay_type: OverlayTextureType);

    fn set_filtering(&mut self, state: i8);

    fn set_lighting(&mut self, state: LightStateType);

    fn set_alpha_type(&mut self, state: AlphaType);

    fn set_color_model(&mut self, state: ColorModelType);

    fn set_zbuffer_state(&mut self, state: i8);

    /// Sets constant alpha
    fn set_alpha_value(&mut self, value: u8);

    /// Gets LowerX, TopY, Width and Height coords of the screen
    fn get_projection_screen_rect(&self) -> super::drawing_3d::ScreenViewPort;
}