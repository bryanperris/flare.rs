use core::{default, sync::atomic::AtomicUsize};
use effect_cone::ConeEffect;
use effect_fall::FallEffect;
use effect_fire::{FireEffect, FireEmitterEffect, FireModel};
use effect_fountain::FountainEffect;
use effect_lightning::{LightningEffect, SphereLightningEffect};
use effect_random_ember::RandomEmberEffect;
use effect_rising_ember::RisingEmberEffect;
use effect_roamer::RoamerEffect;
use effect_water::{WaterEffect, WaterEffectVariant};
use std::{
    env,
    fs::File,
    io::{BufReader, Cursor},
    os::unix::raw::off_t,
    path::{Path, PathBuf},
};

use crate::{
    display_1555, display_4444, display_argb32,
    graphics::{
        bitmap::{self, image_format_ogf::OgfBitmap, Bitmap16, BitmapFormat},
        color_conversion::{alpha_blend, convert_4444_to_32},
        detail_settings,
        generic_bitmap::GenericBitmap16,
    },
    retail::assets::testing::get_d3_hog, testdata,
};

use super::*;

const fn generate_white_green_black_palette() -> [u16; ProcPalette::SIZE] {
    let mut palette = [0u16; ProcPalette::SIZE];

    /* First half: White to Green */
    let mut i = 0;
    while i < 128 {
        let norm = i as f32 / 127.0;
        let ir = ((1.0 - norm) * 31.0) as u16; // Red decreases from 31 to 0
        let ig = 63; // Green stays at max (63 in 16-bit color)
        let ib = ((1.0 - norm) * 31.0) as u16; // Blue decreases from 31 to 0
        palette[i] = OPAQUE_FLAG | (ir << 10) | (ig << 5) | ib;
        i += 1;
    }

    /* Second half: Green to Black */
    let mut i = 0;
    while i < 128 {
        let norm = i as f32 / 127.0;
        let ir = 0; // Red stays at 0
        let ig = ((1.0 - norm) * 63.0) as u16; // Green decreases from 63 to 0
        let ib = 0; // Blue stays at 0
        palette[i + 128] = OPAQUE_FLAG | (ir << 10) | (ig << 5) | ib;
        i += 1;
    }

    palette
}

fn upscale_4x_sharpest(argb_buffer: &[u32], width: usize, height: usize) -> Vec<u32> {
    let new_width = width * 4;
    let new_height = height * 4;
    let mut upscaled_buffer = vec![0; new_width * new_height];

    for y in 0..height {
        for x in 0..width {
            let pixel = argb_buffer[y * width + x];
            
            // Write the pixel to a 4x4 block in the new buffer
            for dy in 0..4 {
                for dx in 0..4 {
                    let new_x = x * 4 + dx;
                    let new_y = y * 4 + dy;
                    upscaled_buffer[new_y * new_width + new_x] = pixel;
                }
            }
        }
    }

    upscaled_buffer
}

fn do_proc_test<F: FnMut() -> BaseEmitter>(
    mut fn_emitter_generation: F,
    p: Option<ProcPalette>,
    clear: bool,
    model: Option<Box<dyn ProceduralModel>>,
) {
    let badapple = File::open(testdata!("kokomi2.ogf")).unwrap();
    let mut reader = BufReader::new(badapple);
    let bitmap = bitmap::image_format_ogf::OgfBitmap::new(&mut reader, bitmap::BitmapFormat::Fmt1555).unwrap();
    let bitmap = crate::common::new_shared_mut_ref(bitmap);

    let detail_settings = DetailSettings {};

    let frame_counter = FrameCounter::new(AtomicUsize::new(0));
    let system_clock = crate::common::StdSystemClock;

    let mut proc_bitmap_builder = ProceduralBitmap16Builder::default();
    let mut proc_bitmap_builder = proc_bitmap_builder.name("test_proc")
        .dest_bitmap(128, 128)
        .detail_settings_ref(crate::common::new_shared_mut_ref(detail_settings))
        .frame_counter_ref(frame_counter.clone())
        .base_bitmap_ref(bitmap)
        .heat(0xFF)
        .system_clock_ref(Arc::new(system_clock));

    if model.is_some() {
        proc_bitmap_builder = proc_bitmap_builder.model(model.unwrap());
    }
        
    if p.is_some() {
        proc_bitmap_builder = proc_bitmap_builder.palette(p.unwrap());
    }

    let mut proc_bitmap = proc_bitmap_builder.build().unwrap();

    let mut emitters: Vec<BaseEmitter> = Vec::new();

    use minifb::{Key, Window, WindowOptions};

    let mut window = Window::new(
        "Procedural Test (Press ESC to close)",
        PROC_SIZE * 4,
        PROC_SIZE * 4,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    let mut time = 0;
    let mut appended = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if clear {
            proc_bitmap.clear_emitters();
            appended = false;
        }

        if !appended {
            emitters.clear();
            emitters.push(fn_emitter_generation());
            proc_bitmap.append_emitters(&mut emitters);
            appended = true;
        }

        proc_bitmap.step(time as f32); //convert_8_to_32

        // let mem = proc_bitmap.memory.as_mut().unwrap();

        // window
        // .update_with_buffer(
        //     &crate::graphics::color_conversion::convert_16_to_32(mem.front_16()),
        //     PROC_SIZE,
        //     PROC_SIZE,
        // )
        // .unwrap();

        window
            .update_with_buffer(
                &crate::graphics::color_conversion::convert_1555_to_32(&proc_bitmap.data()),
                PROC_SIZE,
                PROC_SIZE,
            )
            .unwrap();

        time += 1;
        frame_counter.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }
}

fn random_point<R: Rand>(rand: &mut R) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;

    x = (rand.next_usize() % PROC_SIZE) as f32;
    y = (rand.next_usize() % PROC_SIZE) as f32;

    (x.abs(), y.abs())
}

#[test]
#[function_name::named]
fn procedurals_test() {
    crate::test_common::setup();

    let mut rand = crate::create_rng();

    // display_1555!("Default Palette", &ProcPalette::DEFAULT.table(), 16, 16);

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let (x2, y2) = random_point(&mut rand);

            let effect = FireEffect {
                effect: Box::new(LightningEffect),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 1,
                color: 0xFF,
                size: 1,
                x1: x,
                y1: y,
                x2: x2,
                y2: y2,
            };

            e
        },
        None,
        true,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(SphereLightningEffect),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 1,
                color: 0xFF,
                size: 0xFF,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        true,
        Some(Box::new(FireModel)),
    );

    let p = ProcPalette::from_raw(generate_white_green_black_palette());
    // // display_1555!("New Palette", &p.table(), 16, 16);

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);

            let effect = FireEffect {
                effect: Box::new(RoamerEffect::new(x, y)),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 1,
                color: 0x1F,
                size: 0xFF,
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        Some(p),
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(RandomEmberEffect::default()),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 1,
                color: 0xFF,
                size: 0xFF,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(RisingEmberEffect::default()),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 1,
                color: 0xFF,
                size: 0xFF,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(ConeEffect::default()),
            };
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 5,
                color: 0xFF,
                size: 5,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(FountainEffect::default()),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 5,
                color: 0xFF,
                size: 5,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(FallEffect::<0>::default()),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 5,
                color: 0xFF,
                size: 5,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let effect = FireEffect {
                effect: Box::new(FallEffect::<1>::default()),
            };

            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 0,
                speed: 5,
                color: 0xFF,
                size: 5,
                x1: 128.0 / 2.0,
                y1: 128.0 / 2.0,
                x2: 128.0,
                y2: 128.0,
            };
            e
        },
        None,
        false,
        Some(Box::new(FireModel)),
    );

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let water_effect = water_effects::HeightBlobWaterEffect;
            let mut effect = WaterEffect::new(water_effect);
            effect.set_thickness(4);
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 5,
                speed: 20,
                color: 0,
                size: 10,
                x1: x,
                y1: y,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        None,
        true,
        None,
    );

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let water_effect = water_effects::HeightBlobWaterEffect;
            let mut effect = WaterEffect::new(water_effect);
            effect.set_light(8);
            effect.set_thickness(8);
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 5,
                speed: 25,
                color: 0,
                size: 10,
                x1: x,
                y1: y,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        None,
        true,
        None,
    );

    #[cfg(feature = "retail_testing")]
    {
        let hog = get_d3_hog().unwrap();
        let freaky = hog.borrow_entries()["freakyeye.ogf"].data.as_ref();
        let mut cursor = Cursor::new(freaky);
        let mut reader = BufReader::new(cursor);
        let bitmap = OgfBitmap::new(&mut reader, BitmapFormat::Fmt4444).unwrap();
        let bitmap_ref: Rc<RefCell<dyn Bitmap16>> = Rc::new(RefCell::new(bitmap));

        do_proc_test(
            || {
                let (x, y) = random_point(&mut rand);
                let water_effect = water_effects::HeightBlobWaterEffect;
                let mut effect = WaterEffect::new(water_effect);
                effect.set_light(8);
                effect.set_thickness(4);
                effect.enable_easter_egg(&bitmap_ref);
                let e = BaseEmitter {
                    effect: Some(Box::new(effect)),
                    frequency: 5,
                    speed: 20,
                    color: 0,
                    size: 10,
                    x1: x,
                    y1: y,
                    x2: 0.0,
                    y2: 0.0,
                };
                e
            },
            None,
            true,
            None,
        );
    }

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let water_effect = water_effects::BlobDropsWaterEffect;
            let mut effect = WaterEffect::new(water_effect);
            effect.set_thickness(6);
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 5,
                speed: 30,
                color: 0,
                size: 7,
                x1: x,
                y1: y,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        None,
        true,
        None,
    );

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let water_effect = water_effects::RainDropsWaterEffect;
            let mut effect = WaterEffect::new(water_effect);
            effect.set_thickness(6);
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 8,
                speed: 60,
                color: 0xFF,
                size: 10,
                x1: x,
                y1: y,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        None,
        true,
        None,
    );

    do_proc_test(
        || {
            let (x, y) = random_point(&mut rand);
            let water_effect = water_effects::SineBlobWaterEffect;
            let mut effect = WaterEffect::new(water_effect);
            effect.set_thickness(4);
            let e = BaseEmitter {
                effect: Some(Box::new(effect)),
                frequency: 5,
                speed: 60, // Really height
                color: 0,
                size: 13, // Radius
                x1: x,
                y1: y,
                x2: 0.0,
                y2: 0.0,
            };
            e
        },
        None,
        true,
        None,
    );
}
