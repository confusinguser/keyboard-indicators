use std::collections::HashMap;
use std::time::{Duration, Instant};

use hsv::hsv_to_rgb;
use rand::Rng;
use rgb::{RGB, RGB8};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::keyboard_controller::{KeyboardController, KeyboardControllerMessage};
use crate::core::utils;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct StarfieldModuleOptions {
    pub(crate) background: RGB8,
    pub(crate) target_color: RGB8,
    pub(crate) animation_time: Duration,
    pub(crate) time_variation: f32,
}

impl Default for StarfieldModuleOptions {
    fn default() -> Self {
        Self {
            background: RGB::from(hsv_to_rgb(44., 0.99, 0.14)),
            target_color: RGB::from(hsv_to_rgb(44., 0.99, 0.99)),
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
        mut sender: Sender<KeyboardControllerMessage>,
        module_leds: Vec<Option<u32>>,
        options: StarfieldModuleOptions,
    ) {
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
                            + if options.time_variation != 0. {
                                rand::thread_rng()
                                    .gen_range(-options.time_variation..options.time_variation)
                            } else {
                                0.
                            });
                    if *progress >= 1. {
                        *progress %= 1.;
                    }
                }

                last_update = Instant::now();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }
}

fn animation_curve(background: RGB8, target: RGB8, progress: f32) -> rgb::RGB<u8> {
    if progress < 0.5 {
        utils::interpolate(background, target, progress * 2.)
    } else {
        utils::interpolate(target, background, progress * 2. - 1.)
    }
}
