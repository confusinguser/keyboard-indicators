use std::collections::HashMap;
use std::time::{Duration, Instant};

use hsv::hsv_to_rgb;
use openrgb::data::Color;
use rand::Rng;
use rgb::{ComponentMap, RGB, RGB8};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::keyboard_controller::KeyboardController;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct StarfieldModuleOptions {
    pub(crate) background: RGB8,
    pub(crate) min_currently_in_animation: u32,
    pub(crate) target_color: RGB8,
    pub(crate) probability: f64,
    pub(crate) animation_time: Duration,
    #[serde(default)]
    pub(crate) time_variation: f32,
}

impl Default for StarfieldModuleOptions {
    fn default() -> Self {
        Self {
            background: RGB::from(hsv_to_rgb(44., 0.99, 0.14)),
            min_currently_in_animation: 8,
            target_color: RGB::from(hsv_to_rgb(44., 0.99, 0.99)),
            probability: 0.0,
            animation_time: Duration::from_secs(2),
            time_variation: 0.5,
        }
    }
}

pub(crate) struct StarfieldModule {}

impl StarfieldModule {
    pub fn run(
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        mut sender: Sender<(u32, Color)>,
        module_leds: Vec<Option<u32>>,
        options: StarfieldModuleOptions,
    ) {
        let options = StarfieldModuleOptions::default();
        task_tracker.spawn(async move {
            // The value in the map is how far along the LED has gotten in the animation
            let mut leds_animation: HashMap<u32, f32> = HashMap::new();
            for led in module_leds.iter().flatten() {
                leds_animation.insert(*led, rand::thread_rng().gen_range(0_f32..1.));
            }

            let mut last_update = Instant::now();
            loop {
                if cancellation_token.is_cancelled() {
                    println!("Exiting starfield");
                    break;
                }

                let now = Instant::now();
                for (led, progress) in leds_animation.iter_mut() {
                    KeyboardController::update_led(
                        &mut sender,
                        *led,
                        animation_curve(options.background, options.target_color, *progress),
                    )
                    .await
                    .unwrap();
                    *progress += (now - last_update).as_secs_f32()
                        / (options.animation_time.as_secs_f32()
                            + rand::thread_rng()
                                .gen_range(-options.time_variation..options.time_variation));
                    if *progress >= 1. {
                        *progress %= 1.;
                    }
                }

                last_update = Instant::now();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
}

fn interpolate(from: RGB8, to: RGB8, progress: f32) -> rgb::RGB<u8> {
    let mut out = RGB8::new(0, 0, 0);
    out += from.map(|comp| (comp as f32 * (1. - progress)) as u8);
    out += to.map(|comp| (comp as f32 * progress) as u8);
    out
}

fn animation_curve(background: RGB8, target: RGB8, progress: f32) -> rgb::RGB<u8> {
    if progress < 0.5 {
        interpolate(background, target, progress * 2.)
    } else {
        interpolate(target, background, progress * 2. - 1.)
    }
}
