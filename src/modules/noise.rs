use std::time::{Duration, Instant};

use hsv::hsv_to_rgb;
use noise::NoiseFn;
use rand::random;
use rgb::{RGB, RGB8};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::keyboard_controller::{KeyboardController, KeyboardControllerMessage};
use crate::core::utils;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct NoiseModuleOptions {
    pub(crate) color1: RGB8,
    pub(crate) color2: RGB8,
    /// Per second
    pub(crate) speed: f32,
    /// Divides coordinates by value
    #[serde(default)]
    pub(crate) zoom_in: f32,
}

impl Default for NoiseModuleOptions {
    fn default() -> Self {
        Self {
            color1: RGB::from(hsv_to_rgb(44., 0.99, 0.02)),
            color2: RGB::from(hsv_to_rgb(44., 0.99, 0.15)),
            speed: 0.01,
            zoom_in: 3.,
        }
    }
}

pub(crate) struct NoiseModule {}

impl NoiseModule {
    pub fn run(
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        mut sender: Sender<KeyboardControllerMessage>,
        module_leds: Vec<Option<u32>>,
        _options: NoiseModuleOptions,
    ) {
        task_tracker.spawn(async move {
            let options = NoiseModuleOptions::default();
            let mut depth = 0.;
            let mut last_update = Instant::now();
            let noise = noise::SuperSimplex::new(0);
            let offset: (f64, f64) = (random(), random());

            loop {
                if cancellation_token.is_cancelled() {
                    break;
                }

                let now = Instant::now();
                // If we have two nones back to back, we consider that a line break
                let mut back_to_back_nones = 0;
                let mut current_row = 0;
                let mut first_in_this_row = 0;
                for (i, led) in module_leds.iter().enumerate() {
                    let Some(led) = led else {
                        back_to_back_nones += 1;
                        if back_to_back_nones == 2 {
                            current_row += 1;
                            first_in_this_row = i + 1;
                            back_to_back_nones = 0;
                        }
                        continue;
                    };

                    let x = i - first_in_this_row;
                    KeyboardController::update_led(
                        &mut sender,
                        *led,
                        led_value(
                            &noise,
                            options.color1,
                            options.color2,
                            depth as f64,
                            x as f64 / options.zoom_in as f64 + offset.0,
                            current_row as f64 / options.zoom_in as f64 + offset.1,
                        ),
                    )
                    .await
                    .unwrap();

                    depth += (now - last_update).as_secs_f32() * options.speed;
                    back_to_back_nones = 0;
                }

                last_update = Instant::now();
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
    }
}

fn led_value<T: NoiseFn<f64, 3>>(
    noise: &T,
    background: RGB8,
    target: RGB8,
    depth: f64,
    x: f64,
    y: f64,
) -> rgb::RGB<u8> {
    let interpolation = noise.get([x, y, depth]);
    utils::interpolate(background, target, interpolation as f32)
}
