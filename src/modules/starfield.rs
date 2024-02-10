use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hsv::hsv_to_rgb;
use rand::distributions::{Bernoulli, Distribution};
use rand::Rng;
use rgb::{ComponentMap, RGB, RGB8};
use serde::{Deserialize, Serialize};
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
}

impl Default for StarfieldModuleOptions {
    fn default() -> Self {
        Self {
            background: RGB::from(hsv_to_rgb(44., 0.99, 0.14)),
            min_currently_in_animation: 8,
            target_color: RGB::from(hsv_to_rgb(44., 0.99, 0.99)),
            probability: 0.0,
            animation_time: Duration::from_secs(2),
        }
    }
}

pub(crate) struct StarfieldModule {}

impl StarfieldModule {
    pub fn run(
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
        options: StarfieldModuleOptions,
    ) {
        let options = StarfieldModuleOptions::default();
        task_tracker.spawn(async move {
            let bernoulli = Bernoulli::new(options.probability);
            let Ok(bernoulli) = bernoulli else {
                eprintln!(
                    "Bernoulli distribution creation gave error: {}",
                    bernoulli.unwrap_err()
                );
                return;
            };
            // The value in the map is how far along the LED has gotten in the animation
            let mut currently_in_animation: HashMap<u32, f32> = HashMap::new();
            for led in module_leds.iter().flatten() {
                keyboard_controller
                    .set_led_by_index(*led, options.background)
                    .await
                    .unwrap();
            }

            let mut last_update = Instant::now();
            loop {
                if cancellation_token.is_cancelled() {
                    println!("Exiting starfield");
                    break;
                }
                if bernoulli.sample(&mut rand::thread_rng())
                    || currently_in_animation.len() < options.min_currently_in_animation as usize
                {
                    let random_led =
                        pick_random_led_not_in_animation(&module_leds, &currently_in_animation);
                    if let Some(random_led) = random_led {
                        currently_in_animation.insert(random_led, 0.);
                    }
                }

                let now = Instant::now();
                for (led, progress) in currently_in_animation.iter_mut() {
                    keyboard_controller
                        .set_led_by_index(
                            *led,
                            animation_curve(options.background, options.target_color, *progress),
                        )
                        .await
                        .unwrap();
                    *progress +=
                        (now - last_update).as_secs_f32() / options.animation_time.as_secs_f32();
                }

                currently_in_animation.retain(|_, progress| *progress <= 1.);

                last_update = Instant::now();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
}

fn pick_random_led_not_in_animation(
    module_leds: &Vec<Option<u32>>,
    currently_in_animation: &HashMap<u32, f32>,
) -> Option<u32> {
    if currently_in_animation.len() >= module_leds.len() {
        return None;
    }
    let random_led =
        rand::thread_rng().gen_range(0..(module_leds.len() - currently_in_animation.len()) as u32);
    let mut count = 0;
    for led in module_leds {
        // TODO guarantee no None in it
        let led = led.unwrap();
        if !currently_in_animation.contains_key(&led) {
            if count >= random_led {
                return Some(led);
            }
            count += 1;
        }
    }
    None
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
