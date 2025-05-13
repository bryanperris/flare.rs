

use core::borrow::Borrow;
use std::{ascii, rc::Rc};

use bitflags::bitflags;

use crate::{common::unsigned_safe_sub, gr_rgb, graphics::{ddgr_color, drawing_2d::font::{FontGlyph, GlyphClipRect}, rendering::{AlphaType, AlphaTypeFlags, Renderer}, GR_BLACK, GR_COLOR_CHAR}, string::D3String, string_common::convert_to_ascii_slice};

use super::font::{Font, FontGraphic, FontTemplate};

pub enum TextOpcodes {
    Text {x: usize, y: usize, text: D3String},
    CenteredText{ x: usize, y: usize, text: D3String},
    SetColor (ddgr_color),
    FancyColor (ddgr_color),
    SetFont (Rc<FontGraphic>),
    SetAlpha (u8),
    SetFlags (TextFlags),
    Scale (f32),
}

pub enum TextFormat {
    Char = 2,
    /// value mulitplied to formatting value in string.
    Scalar = 4
}

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TextFlags: u8 {
        const None  =       0b00000000;
        const Saturate =    0b00000001;
        const Shadow =      0b00000010;
    }
}

macro_rules! apply_kerning {
    ($usize_var:expr, $isize_var:expr) => {
        {
            if $isize_var < 0 {
                // If the isize is negative, subtract its absolute value from the usize
                let sub_value = $isize_var.abs() as usize;
                if sub_value > $usize_var {
                    0
                } else {
                    $usize_var - sub_value
                }
            } else {
                // If the isize is not negative, add its value to the usize
                let add_value = $isize_var as usize;
                $usize_var + add_value
            }
        }
    };
}

// Embedded formatting opcodes
const FORMAT_COLOR: char = 1 as char;
const FORMAT_CHAR: char = 2 as char;
const FORMAT_SCALAR: usize = 4;

pub struct RenderedTextRect{
    pub left: usize,
    pub right: usize,
    pub top: usize,
    pub bottom: usize,
}

impl Default for RenderedTextRect {
    fn default() -> Self {
        Self { 
            left: Default::default(), 
            right: Default::default(), 
            top: Default::default(), 
            bottom: Default::default(), 
        }
    }
}

pub struct RenderedTextBuf {
    spacing: usize,
    formatted_text: Vec<TextOpcodes>,
    font: Option<Rc<FontGraphic>>,
    alpha: u8,
    alpha_type: AlphaTypeFlags,
    use_shadowing: bool,
    line_spacing: usize,
    clip: RenderedTextRect,
    rect: RenderedTextRect,
    tab_spacing: usize,
    colors: [ddgr_color; 2],
    scale: f32,
    color: ddgr_color,
}

struct RenderedTextChar {
    op: TextOpcodes,
    scale: f32
}


///	It will break the source buffer up into lines (seperated by /n) of size width or smaller (in pixels).
pub fn text_word_wrap(text: &Box<[u8]>, width: usize, font: &Rc<Font>, spacing: usize) -> Box<[u8]> {
    let mut wrapped_text = text.clone().into_vec();

    let mut done = false;

    let mut start_line_offset = 0;
    let mut last_word_offset = 0;
    let mut last_pos = 0;

    while (!done) {
        let mut curr_width = 0;
        let mut index = 0;
        let mut num_words_on_lines = 0;

        while (curr_width <= width || num_words_on_lines == 0) {
            match wrapped_text[index] {
                b'\0' => {
                    done = true;
                },
                b'\n' => {
                    curr_width = width + 1;
                },
                b' ' => {
                    last_word_offset = index;
                    num_words_on_lines += 1;
                    break;
                },
                _ => {
                    {}
                }
            }

            if wrapped_text[index] != b'\0' && wrapped_text[index] != b'\n' {
                curr_width += font.get_char_width(wrapped_text[index] as usize);
                curr_width += spacing;
            }

            index += 1;
        }

        if !done {
            wrapped_text[last_word_offset] = b'\n';
        }

        start_line_offset += (last_word_offset - start_line_offset + 1);
        last_word_offset = start_line_offset;
    }

    wrapped_text.into_boxed_slice()
}

////	This function goes hand-in-hand with text_word_wrap.  Given a buffer of data it will fill in
////	the dest buffer until it hits a /n or /0.  It returns the position of the next line,
////    or None if it's done with the buffer (it hit a /0).
pub fn text_copy_text_line(text: Box<[u8]>) -> (Box<[u8]>, Option<usize>) {
    let mut end_index = text.len();

    for (i, &byte) in text.iter().enumerate() {
        if byte == b'\n' || byte == b'\0' {
            end_index = i;
            break;
        }
    }

    let line: Box<[u8]> = text[..end_index].to_vec().into_boxed_slice();

    ( line, 
        
        if end_index + 1 < text.len() {
            Some(end_index + 1)
        }
        else {
            None
        }
    )
}

//	Given a width (in pixels), and a string, this function will truncate the string
//	to at most width pixels.  If the end parameter is not 0, then that char is attached to
//  the end of the string if it has to clip(the char's width is taken into consideration).
//  It is based off the current font.  if horizont_ratio is given it is used to correct for
//  possible different sized hud fonts.  For instance, if this string is going to be printed on
//  the hud, then you should always pass (DEFAULT_HUD_WIDTH/((float)*Game_window_w)) as the
//  horizont_ratio paramter.
// pub fn text_clip_string(text: &Box)
// XXX: D3 doesn't make calls to this function

impl Default for RenderedTextBuf {
    fn default() -> Self {
        Self { 
            spacing: 1, 
            formatted_text: Default::default(), 
            font: None, 
            alpha: 255, 
            alpha_type: AlphaTypeFlags::Texture & AlphaTypeFlags::Constant,
            use_shadowing: false, 
            line_spacing: 1, 
            clip: RenderedTextRect::default(),
            rect: RenderedTextRect::default(),
            colors: Default::default(), 
            tab_spacing: 1,
            scale: 1.0f32,
            color: GR_BLACK
        }
    }
}

impl RenderedTextBuf {
    fn flush<T: Renderer>(&mut self, renderer: &mut T) {
        self.render(renderer);
        self.formatted_text.clear();
    }

    // This needs to handle the strong types: TextOpcodes
    fn get_text_line_width(&self, text: &D3String, template_override: Option<FontTemplate>) -> usize {
        let mut rgb_define_mode = 0;
        let mut line_width = 0usize;
        let mut max_width = 0usize;

        let fn_char_width = |ch| -> usize {
            match template_override.as_ref() {
                Some(t) => {
                    t.character_width(ch)
                },
                _ => {
                    self.font.as_ref().unwrap().get_char_width(ch)
                }
            }
        };

        let fn_kern_spacing = |ch1, ch2| -> isize {
            match template_override.as_ref() {
                Some(t) => {
                    t.character_spacing(ch1, ch2) as isize
                },
                _ => {
                    self.font.as_ref().unwrap().get_font().get_kerned_spacing(ch1, ch2)
                }
            }
        };
    
        let mut i = 0;
        while i < text.len() {
            let ch1 = text[i + 0] as char;
            let ch2 = if i + 1 < text.len() {
                text[i + 1] as char
            }
            else {
                '\0'
            };
    
            // note that if we hit the GR_COLOR_CHAR then the next three values should
            // not count when defining the width of the line.
    
            if rgb_define_mode == 3 {
                rgb_define_mode = 0;
            }
            else if ch1 == FORMAT_COLOR {
                rgb_define_mode = 1;
            }
    
            if rgb_define_mode == 0 {
                match ch1 {
                    '\t' => {
                        let space_width = (fn_char_width(' ' as u8 as usize) + self.spacing) * self.tab_spacing;
                        line_width = (line_width + space_width) / space_width * space_width;
                    },
                    FORMAT_CHAR => {
                        if i + 1 >= text.len() {
                            break;
                        }

                        line_width = text[i + 1] as usize * FORMAT_SCALAR;
                        i += 1;
                    },
                    '\n' => {
                        if line_width > max_width {
                            max_width = line_width;
                        }
                        line_width = 0;
                    },
                    _ => {
                        if ch2 != '\0' && ch2 != '\n' && ch2 != '\t' && ch2 != FORMAT_CHAR {
                            let width = fn_char_width(ch1 as usize);

                            line_width += self.spacing;

                            apply_kerning!(line_width, fn_kern_spacing(ch1 as usize, ch2 as usize));
                        }
                    }
                }
            }
            else {
                rgb_define_mode += 1;
            }

            i += 1;
        }

        if line_width > max_width {
            max_width = line_width;
        }
    
        if max_width != 0 {
            return unsigned_safe_sub(max_width, self.spacing);
        }
        else {
            return 0;
        }
    }

    fn set_clip(&mut self, clip: RenderedTextRect) {
        self.clip = clip;
    }

    fn get_clip(&self) -> &RenderedTextRect {
        &self.clip
    }

    fn append_color(&mut self, color: ddgr_color) {
        self.formatted_text.push(TextOpcodes::SetColor(color));
        self.color = color;
    }

    fn get_color(&self) -> ddgr_color {
        self.color
    }

    fn append_font_scale(&mut self, scale: f32) {
        self.formatted_text.push(TextOpcodes::Scale(scale));
        self.scale = scale;
    }

    fn append_fancy_color(&mut self, c1: ddgr_color) {
        self.formatted_text.push(TextOpcodes::FancyColor(c1))
    }

    fn set_alpha(&mut self, alpha: u8) {
        self.formatted_text.push(TextOpcodes::SetAlpha(alpha));
    }

    fn get_alpha(&self) -> u8 {
        self.alpha
    }

    fn append_flags(&mut self, flags: TextFlags) {
        self.formatted_text.push(TextOpcodes::SetFlags(flags));
    }

    fn append_font(&mut self, fontg: &Rc<FontGraphic>) {
        self.font = Some(Rc::clone(fontg));
        self.spacing = fontg.get_font().get_tracking() + 1;
        self.formatted_text.push(TextOpcodes::SetFont(Rc::clone(fontg)));
    }

    fn get_font(&self) -> &Option<Rc<FontGraphic>> {
        &self.font
    }

    fn append_text(&mut self, text: D3String, x: usize, y: usize) {
        self.formatted_text.push(TextOpcodes::Text { x: x, y: y, text: text});
    }

    fn append_text_centered(&mut self, text: D3String, x: usize, y: usize) {
        self.formatted_text.push(TextOpcodes::CenteredText { x: x, y: y, text: text});
    }

    pub fn render<T: Renderer>(&self, renderer: &mut T) {
        /* Setup rendering of the text */
        renderer.set_texture_type(crate::graphics::rendering::TextureType::Linear);
        renderer.set_overlay_type(crate::graphics::rendering::OverlayTextureType::None);
        renderer.set_filtering(0);
        renderer.set_lighting(crate::graphics::rendering::LightStateType::FlatGouraud);
        renderer.set_alpha_type(AlphaType::TEXTURE | AlphaType::CONSTANT);
        renderer.set_color_model(crate::graphics::rendering::ColorModelType::Mono);
        renderer.set_zbuffer_state(0);
        renderer.set_alpha_value(self.alpha);

        let mut position = 0usize;

        // TODO: The original code will modify the grtext properties, but we don't want that!
        let mut font = self.font.as_ref().unwrap().clone();
        let mut text_color: ddgr_color = self.color;
        let mut scale = self.scale;

        for opcode in &self.formatted_text {
            match opcode {
                TextOpcodes::SetColor(v) => {
                    renderer.set_flat_color(v.to_owned());
                    text_color = v.to_owned();
                },
                TextOpcodes::SetFlags(v) => {
                    renderer.set_alpha_type(if v.contains(TextFlags::Saturate) {
                        AlphaType::TEXTURE
                    }
                    else {
                        AlphaType::TEXTURE | AlphaType::CONSTANT
                    });
                },
                TextOpcodes::FancyColor(v) => {
                    renderer.set_flat_color(v.to_owned());
                    text_color = v.to_owned();
                },
                TextOpcodes::SetAlpha(v) => {
                    renderer.set_alpha_value(v.to_owned());
                },
                TextOpcodes::SetFont(v) => {
                    font = v.clone();
                },
                TextOpcodes::Text { x, y, text } => {
                    if self.use_shadowing {
                        renderer.set_flat_color(0);
                        self.render_string(renderer, &font, x + 1, y + 1, scale, &text);
                        renderer.set_flat_color(text_color)
                    }

                    self.render_string(renderer, &font, *x, *y, scale, &text);
                },
                TextOpcodes::CenteredText { x, y, text } => {
                    let x = x + self.clip.left + (self.clip.right - self.clip.left) / 2 - self.get_text_line_width(&text, None) / 2;

                    if self.use_shadowing {
                        renderer.set_flat_color(0);
                        self.render_string(renderer, &font, x + 1, y + 1, scale, &text);
                        renderer.set_flat_color(text_color)
                    }

                    self.render_string(renderer, &font, x, *y, scale, &text);
                },
                TextOpcodes::Scale(v) => {
                    scale = v.to_owned();
                }
            }
        }
    }

    fn render_string<T: Renderer>(&self, renderer: &mut T, font_graphic: &FontGraphic, x: usize, y: usize, scale: f32, text: &D3String) {
        let mut cur_x = x;
        let mut cur_y = y;

        let mut lines: Vec<D3String> = Vec::new();
        let mut start = 0;
    
        for (i, &b) in text.iter().enumerate() {
            if b == b'\n' {
                lines.push(D3String::from_slice(&text[start..i]));
                start = i + 1;
            }
        }
        // Add the last line if there's no trailing newline
        if start < text.len() {
            lines.push(D3String::from_slice(&text[start..]));
        }

        for line in &lines {
            let line_width = self.get_text_line_width(&line, None);

            let gx = cur_x;
            let gy = cur_y;

            let mut clipped = 0;

            if self.clip.top > (gy + font_graphic.get_font().get_height()) || self.clip.bottom < gy {
                clipped = 2;
            }
            else if self.clip.left > (gx + line_width) || self.clip.right < gx {
                clipped = 2;
            }

            if clipped != 2 {
                if self.clip.left > gx || self.clip.right < (gx + line_width) {
                    clipped = 1;
                }

                if self.clip.top > gy || self.clip.bottom < (gy + font_graphic.get_font().get_height()) {
                    clipped = 1;
                }

                self.render_text_line(renderer, font_graphic, gx, gy, scale, clipped != 0, line);
            }

            cur_y += self.spacing;
            cur_y += self.get_font().as_ref().unwrap().get_height();
            cur_x = self.clip.left;
        }
    }

    fn render_text_line<T: Renderer>(&self, renderer: &mut T, font_graphic: &FontGraphic, x: usize, y: usize, scale: f32, do_clip: bool, text: &D3String) {
        /*	by clipping, we should first determine what our vertical clipping is.  then
              go through each character in the line and determine what is totally clipped,
              partially clipped and by how much, and not clipped at all and draw accordingly
        */

        let font = font_graphic.get_font();
        let h = font.get_height();
        let mut ch_y = 0;
        let mut ch_h = h;
        let mut draw_y = y;

        //	determine each character bitmap y and height to render
        if do_clip {
            if (self.clip.top >= y) {
                ch_y = unsigned_safe_sub(self.clip.top, y);
                draw_y = self.clip.top;
            }

            if self.clip.bottom < unsigned_safe_sub(y, font.get_height()) {
                ch_h = self.clip.bottom - y;
            }

            /* do this to clip both top and bottom */
            ch_h = unsigned_safe_sub(ch_h, ch_y);
        }

        let mut cur_x = x;

        let mut i = 0;

        while i < text.len() {
            let ch1 = text[i + 0];
            let ch2 = if i + 1 < text.len() {
                text[i + 1]
            }
            else {
                '\0' as u8
            };

            let w = font.get_char_width(ch1 as usize);
 
            match ch1 as char {
                FORMAT_COLOR => {
                    if i + 3 > text.len() {
                        panic!("bad string provided");
                    }

                    let r = text[i + 1] as u32;
                    let g = text[i + 2] as u32;
                    let b = text[i + 3] as u32;
                    let col: ddgr_color = gr_rgb!(r, g, b);

                    renderer.set_flat_color(col);

                    i += 3;
                },
                '\t' => {
                    let space_width = 
                        (font.get_char_width(' ' as u8 as usize) + self.spacing) *
                        self.tab_spacing;

                    cur_x = (cur_x + space_width) / space_width * space_width;
                }
                // Clipping check
                c if (cur_x + w) < self.clip.left || cur_x > self.clip.right => {
                    cur_x += self.spacing + w;
                },
                FORMAT_CHAR => {
                    if i + 1 > text.len() {
                        panic!("bad string provided");
                    }

                    cur_x = x + (text[i + 1] as usize * FORMAT_SCALAR);
                    i += 1;
                },
                ' ' => {
                    cur_x += self.spacing;
                    cur_x += font.get_char_width(' ' as u8 as usize);

                    if ch2 != 0 {
                        apply_kerning!(cur_x, font.get_kerned_spacing(ch1 as usize, ch2 as usize));
                    }
                },
                _ => {
                    let mut ch_x = 0;
                    let mut ch_w = w;
                    let mut draw_x = cur_x;

                    let mut glyph = FontGlyph::default();
                    glyph.character_index = ch1 as usize;
                    glyph.x = draw_x;
                    glyph.y = draw_y;
                    glyph.scale_x = scale;
                    glyph.scale_y = scale;

                    if do_clip {
                        if self.clip.left > cur_x {
                            ch_x = self.clip.left - cur_x;
                            draw_x = self.clip.left;
                        }

                        if self.clip.right < (cur_x + w) {
                            ch_w = self.clip.right - cur_x;
                        }

                        ch_w = unsigned_safe_sub(ch_w, ch_x);

                        if ch_x == 0 && ch_w == w && ch_y == 0 && ch_h == h {
                            glyph.clip = None;
                        }
                        else {
                            glyph.clip =Some(GlyphClipRect {
                                x: ch_x,
                                y: ch_y,
                                w: ch_w,
                                h: ch_h
                            });
                        }
                    }

                    cur_x = glyph.compute_drawing_rect(font_graphic);
                    cur_x += self.spacing;

                    if ch2 != 0 {
                        apply_kerning!(cur_x, font_graphic.get_font().get_kerned_spacing(ch1 as usize, ch2 as usize));
                    }

                    // Draw the glyph
                    renderer.draw_font_char(font_graphic, &glyph)
                }
            }

            i += 1;
        }
    }
}

// TODO: Profanity Filter be moved somewhere else

#[cfg(test)]
pub mod tests {
    use std::{env, fs::File, io::{BufReader, Cursor}, os::unix::raw::off_t, path::{Path, PathBuf}};
    use crate::{display_1555, display_4444, display_argb32, graphics::{bitmap::Bitmap16, color_conversion::{alpha_blend, convert_4444_to_32}}, retail::assets::testing::get_d3_hog};
    use function_name::named;

    use super::*;
    
    struct TestingRenderer {
        fake_fb: Vec<u32>,
        width: usize,
        height: usize,
    }

    impl TestingRenderer {
        fn new() -> Self {
            Self {
                fake_fb: vec![0u32; 640 * 480],
                width: 640,
                height: 480
            }
        }

        fn show_render(&self) {
            display_argb32!("TestingRenderer::show_render", self.fake_fb.as_slice(), self.width, self.height);
        }

        fn clear(&mut self) {
            self.fake_fb = vec![0u32; 640 * 480];
        }

        fn blend(
            &mut self,
            chunk: &Vec<u32>,
            x1: usize,
            y1: usize,
            x2: usize,
            y2: usize,
        ) {
            // Ensure x2 and y2 are within bounds
            let x2 = x2;
            let y2 = y2;
        
            // Ensure x1 and y1 are valid
            let x1 = x1.min(self.width);
            let y1 = y1.min(self.height);
        
            // Calculate chunk dimensions
            let chunk_width = x2 - x1;
            let chunk_height = y2 - y1;
        
            // Iterate over the chunk and blend it into the fake_fb
            for y in 0..chunk_height {
                for x in 0..chunk_width {
                    // (cur_y + tex_u) * tex_width * (cur_x + tex_v)
                    let chunk_index = y * chunk_width + x;

                    // (cur_y + offset_y) * fb_width + (cur_x + offset_x)
                    let fb_index = (y + y1) * self.width + (x + x1);
                    
                    self.fake_fb[fb_index] = alpha_blend(chunk[chunk_index], self.fake_fb[fb_index]);
                }
            }
        }
    }

    impl Renderer for TestingRenderer {
        fn set_flat_color(&mut self, color: ddgr_color) {

        }
    
        fn draw_font_char(&mut self, font_graphic: &FontGraphic, glyph: &FontGlyph) {
            let char_bitmap = font_graphic.clone_char_bitmap(glyph.character_index);

            self.blend(&convert_4444_to_32(char_bitmap.0.as_ref()),
                glyph.draw_rect.x1, 
                glyph.draw_rect.y1, 
                glyph.draw_rect.x2,
                glyph.draw_rect.y2);
        }
        
        fn set_texture_type(&mut self, texture_type: crate::graphics::rendering::TextureType) {

        }
        
        fn set_overlay_type(&mut self, overlay_type: crate::graphics::rendering::OverlayTextureType) {

        }
        
        fn set_filtering(&mut self, state: i8) {

        }
        
        fn set_lighting(&mut self, state: crate::graphics::rendering::LightStateType) {

        }
        
        fn set_alpha_type(&mut self, state: AlphaType) {

        }
        
        fn set_color_model(&mut self, state: crate::graphics::rendering::ColorModelType) {

        }
        
        fn set_zbuffer_state(&mut self, state: i8) {

        }
        
        fn set_alpha_value(&mut self, value: u8) {

        }
        
        fn get_projection_screen_rect(&self) -> crate::graphics::drawing_3d::ScreenViewPort {
            todo!()
        }
    }

    #[test]
    #[named]
    #[cfg(feature = "retail_testing")]
    fn test_graphical_test_hello_line() {
        crate::test_common::setup();

        let hog = get_d3_hog().unwrap();

        // println!("hog: {}", hog);
        
        let font_data = hog.borrow_entries()["hihud.fnt"].data.as_ref();
        let mut cursor = Cursor::new(font_data);
        let mut reader = BufReader::new(&mut cursor);

        let font = Font::new_from_steam("hihud".to_string(), &mut reader).unwrap();
        let grfont = FontGraphic::new(font);

        // for b in grfont.get_bitmaps() {
        //     match b.format() {
        //         crate::graphics::bitmap::BitmapFormat::Fmt1555 => {
        //             display_1555!(function_name!(), b.data(), b.width(), b.height());
        //         },
        //         _ => {
        //             display_4444!(function_name!(), b.data(), b.width(), b.height());
        //         }
        //     }
        // }
        
        let mut grtext = RenderedTextBuf {
            clip: RenderedTextRect {
                top: 0,
                right: 480,
                left: 0,
                bottom: 640
            },
            spacing: 2, 
            ..Default::default()
        };

        let mut renderer = TestingRenderer::new();

        let text: D3String = "Hello!".into();

        grtext.render_text_line(&mut renderer, &grfont, 0, 0, 1.0, true, &text);

        renderer.show_render();

        renderer.clear();

        grtext.render_text_line(&mut renderer, &grfont, 0, 0, 1.0, false, &text);

        renderer.show_render();
        
    }

    #[test]
    #[named]
    #[cfg(feature = "retail_testing")]
    fn test_graphical_test_helloworld() {
        crate::test_common::setup();

        let hog = get_d3_hog().unwrap();
        
        let font_data = hog.borrow_entries()["hihud.fnt"].data.as_ref();
        let mut cursor = Cursor::new(font_data);
        let mut reader = BufReader::new(&mut cursor);

        let font = Font::new_from_steam("hihud".to_string(), &mut reader).unwrap();
        let grfont = FontGraphic::new(font);
        
        let mut grtext = RenderedTextBuf {
            clip: RenderedTextRect {
                top: 0,
                right: 480,
                left: 0,
                bottom: 640
            },
            font: Some(grfont),
            spacing: 2, 
            ..Default::default()
        };

        grtext.append_text_centered("Hello World!".into(), 0, 0);
        grtext.append_text_centered("How are you?".into(), 0, 20);

        let mut renderer = TestingRenderer::new();

        grtext.render(&mut renderer);

        renderer.show_render();
    }
}
