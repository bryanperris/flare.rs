use std::default;

use d3_core::{
    graphics::drawing_3d::{
        Camera, ClippingCode, Point3, RenderSetupState, ScreenViewPort,
        legacy_soft::SoftRenderSetup,
    },
    math::{
        matrix::{Matrix, Matrix4},
        vector::Vector,
    },
};
use egui::{TextureOptions, Ui};
use euc::{Buffer2d, LineTriangleList, Pipeline, Target};
use minifb::{Key, Window, WindowOptions};
use once_cell::sync::Lazy;
use rend_soft_options::SoftRenderOptions;
use vek::{Mat4, Rgba, Vec3, Vec4};

mod rend_soft_options;
mod ui;

struct Cube {
    mvp: Mat4<f32>,
}

impl<'r> Pipeline<'r> for Cube {
    type Vertex = (Vec4<f32>, Rgba<f32>);
    type VertexData = Rgba<f32>;
    type Primitives = LineTriangleList;
    type Pixel = u32;
    type Fragment = Rgba<f32>;

    #[inline(always)]
    fn vertex(&self, (pos, color): &Self::Vertex) -> ([f32; 4], Self::VertexData) {
        ((self.mvp * *pos).into_array(), *color)
    }

    #[inline(always)]
    fn fragment(&self, color: Self::VertexData) -> Self::Fragment {
        color
    }

    fn blend(&self, _: Self::Pixel, color: Self::Fragment) -> Self::Pixel {
        u32::from_le_bytes((color * 255.0).as_().into_array())
    }
}

const R: Rgba<f32> = Rgba::new(1.0, 0.0, 0.0, 1.0);
const Y: Rgba<f32> = Rgba::new(1.0, 1.0, 0.0, 1.0);
const G: Rgba<f32> = Rgba::new(0.0, 1.0, 0.0, 1.0);
const B: Rgba<f32> = Rgba::new(0.0, 0.0, 1.0, 1.0);

const D3_POINTS: &[Point3] = &[
    Point3::new(-1.0, -1.0, -1.0),
    Point3::new(-1.0, -1.0, 1.0),
    Point3::new(-1.0, 1.0, -1.0),
    Point3::new(-1.0, 1.0, 1.0),
    Point3::new(1.0, -1.0, -1.0),
    Point3::new(1.0, -1.0, 1.0),
    Point3::new(1.0, 1.0, -1.0),
    Point3::new(1.0, 1.0, 1.0),
];

const VERT_COLORS: &[Rgba<f32>] = &[R, Y, G, B, B, G, Y, R];

const INDICES: &[usize] = &[
    0, 3, 2, 0, 1, 3, // -x
    7, 4, 6, 5, 4, 7, // +x
    5, 0, 4, 1, 0, 5, // -y
    2, 7, 6, 2, 3, 7, // +y
    0, 6, 4, 0, 2, 6, // -z
    7, 1, 5, 3, 1, 7, // +z
];

static FLATTEN_CUBE: Lazy<Box<[Point3]>> = Lazy::new(|| {
    let v: Vec<Point3> = INDICES.iter().map(|&i| D3_POINTS[i]).collect();
    v.into_boxed_slice()
});

// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
// #![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "D3 Playbox",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<D3PlayboxApp>::default())
        }),
    )
}

struct D3PlayboxApp {
    backbuffer: Option<egui::TextureHandle>,
    width: usize,
    height: usize,
    color: Buffer2d<u32>,
    depth: Buffer2d<f32>,

    vert_buffer: Vec<(Vec4<f32>, Rgba<f32>)>,
    vert_buffer_soft: Vec<(Vec4<f32>, Rgba<f32>)>,

    // Camera Controls
    user_rotate_yaw: i32,
    user_rotate_pitch: i32,
    user_pan_z: i32,

    // D3 Rendering
    soft_setup: SoftRenderSetup,
    d3_rend_soft_options: SoftRenderOptions,
}

impl Default for D3PlayboxApp {
    fn default() -> Self {
        Self {
            backbuffer: None,
            width: 800,
            height: 600,
            color: Buffer2d::fill([800, 600], 0),
            depth: Buffer2d::fill([800, 600], 1.0),

            vert_buffer: Vec::new(),
            vert_buffer_soft: Vec::new(),

            user_rotate_yaw: 0,
            user_rotate_pitch: 0,
            user_pan_z: 0,
            d3_rend_soft_options: SoftRenderOptions::default(),

            soft_setup: SoftRenderSetup {
                aspect_override: None,
                aspect: 0.0,
                window_width: 0,
                window_height: 0,
                window_width_2: 0.0,
                window_height_2: 0.0,
                clipper_custom: None,
                clipper_plane_point: Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                xform_pipeline: Default::default(),
                xform: Matrix4::identity(),
                clipper_far_z: 100.0,
            },
        }
    }
}

impl D3PlayboxApp {
    fn load_scene(&mut self) {}

    fn render_3d(&mut self, ui: &mut Ui) {
        // Build the actual vertex list
        self.vert_buffer.clear();

        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.user_rotate_yaw = self.user_rotate_yaw.wrapping_add(15);
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.user_rotate_yaw = self.user_rotate_yaw.wrapping_sub(15);
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.user_rotate_pitch = self.user_rotate_pitch.wrapping_add(15);
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.user_rotate_pitch = self.user_rotate_pitch.wrapping_sub(15);
        }

        if ui.input(|i| i.key_pressed(egui::Key::Z)) {
            self.user_pan_z = self.user_pan_z.wrapping_add(15);
        }

        if ui.input(|i| i.key_pressed(egui::Key::A)) {
            self.user_pan_z = self.user_pan_z.wrapping_sub(15);
        }

        let far_z = 100.0;

        let projection =
            Mat4::perspective_fov_lh_zo(1.3, self.width as f32, self.height as f32, 0.01, far_z);
        let camera_position = Vec3::new(0.0, 0.0, 3.0 + (self.user_pan_z as f32 * 0.0005));
        let scaling = Vec3::new(1.0, -1.0, 1.0);

        let camera_rot = Mat4::rotation_x(self.user_rotate_pitch as f32 * 0.0005)
            * Mat4::rotation_y(self.user_rotate_yaw as f32 * 0.0005)
            * Mat4::rotation_z(0.0);

        if self.d3_rend_soft_options.enable {
            let mut camera = Camera::default();

            camera.position = camera_position.to_owned().into();
            camera.transformation = camera_rot.to_owned().into();
            camera.orientation = camera_rot.to_owned().into();
            camera.scale = scaling.to_owned().into();

            self.soft_setup.on_frame_start(
                &ScreenViewPort {
                    x: 0,
                    y: 0,
                    width: self.width,
                    height: self.height,
                    aspect: 1.3,
                },
                &camera,
            );
        }

        let view = Mat4::<f32>::translation_3d(camera_position)
            * camera_rot
            * Mat4::<f32>::scaling_3d(scaling);
        let mvp: Mat4<f32> = projection * view;

        let d3_soft_mvp = if self.d3_rend_soft_options.enable {
            self.soft_setup.xform_pipeline.compute_final_transform() * projection
        } else {
            Mat4::identity()
        };

        let points: Vec<Point3> = FLATTEN_CUBE
            .iter()
            .filter_map(|p| {
                let mut p = p.to_owned();
                p.compute_clipcode(far_z, &self.soft_setup.clipper_custom);
                Some(p)
            })
            .collect();

        let clipped = if self.d3_rend_soft_options.enable && self.d3_rend_soft_options.use_clip {
            // Update the soft state
            self.soft_setup.aspect = self.width as f32 / self.height as f32;
            self.soft_setup.window_width = self.width;
            self.soft_setup.window_height = self.height;
            self.soft_setup.window_width_2 = self.width as f32 / 2.0;
            self.soft_setup.window_height_2 = self.height as f32 / 2.0;

            // Software clip
            let mut cc_or = ClippingCode::empty();
            let mut cc_and = &mut ClippingCode::empty();

            if self.d3_rend_soft_options.use_clip_top {
                cc_or.insert(ClippingCode::OFF_TOP);
            }
            if self.d3_rend_soft_options.use_clip_bottom {
                cc_or.insert(ClippingCode::OFF_BOT);
            }
            if self.d3_rend_soft_options.use_clip_left {
                cc_or.insert(ClippingCode::OFF_LEFT);
            }
            if self.d3_rend_soft_options.use_clip_right {
                cc_or.insert(ClippingCode::OFF_RIGHT);
            }
            if self.d3_rend_soft_options.use_clip_far {
                cc_or.insert(ClippingCode::OFF_FAR);
            }

            self.soft_setup
                .clipper_clip_polygon(points, &mut cc_or, &mut cc_and)
        } else {
            points
        };

        for i in 0..clipped.len() {
            let p = clipped[i];
            let c = VERT_COLORS[i & (VERT_COLORS.len() - 1)];

            let p: Vec4<f32> = p.into();

            self.vert_buffer_soft.push((p.to_owned(), G));

            self.vert_buffer.push((p, c));
        }

        self.color.clear(0);
        self.depth.clear(1.0);

        if self.vert_buffer.len() > 0 {
            // Render the main scene
            Cube { mvp }.render(
                self.vert_buffer.as_slice(),
                &mut self.color,
                &mut self.depth,
            );

            // Render the D3 soft scene
            if self.d3_rend_soft_options.enable {
                Cube { mvp: d3_soft_mvp }.render(
                    self.vert_buffer_soft.as_slice(),
                    &mut self.color,
                    &mut self.depth,
                );
            }
        }
    }
}

impl eframe::App for D3PlayboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.width = ctx.screen_rect().size().x as usize;
        self.height = ctx.screen_rect().size().y as usize;

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Legacy T&L", |ui| {
                    ui.label("D3 Legacy T&L:");
                    ui.checkbox(
                        &mut self.d3_rend_soft_options.enable,
                        "Enable D3 Software T&L",
                    );
                    ui.checkbox(
                        &mut self.d3_rend_soft_options.use_clip,
                        "Enable Clipping",
                    );
                    ui.checkbox(&mut self.d3_rend_soft_options.use_clip_top, "Clip Top");
                    ui.checkbox(
                        &mut self.d3_rend_soft_options.use_clip_bottom,
                        "Clip Bottom",
                    );
                    ui.checkbox(&mut self.d3_rend_soft_options.use_clip_left, "Clip Left");
                    ui.checkbox(&mut self.d3_rend_soft_options.use_clip_right, "Clip Right");
                    ui.checkbox(&mut self.d3_rend_soft_options.use_clip_far, "Clip Far");
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_3d(ui);

            ui.heading("D3 Software Test");

            let rgba_slice: &[u8] = bytemuck::cast_slice(self.color.raw());
            let image = egui::ColorImage::from_rgba_unmultiplied([800, 600], rgba_slice);

            // Create or update the texture
            if let Some(ref mut tex) = self.backbuffer {
                tex.set(image, TextureOptions::NEAREST);
                ui.image(&*tex);
            } else {
                self.backbuffer = Some(ctx.load_texture("image", image, Default::default()));
            }
        });
    }
}
