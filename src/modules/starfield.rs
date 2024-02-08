use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::distributions::{Bernoulli, Distribution};
use rand::Rng;
use rgb::{ComponentMap, RGB8};
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

pub(crate) struct StarfieldModule {}

impl StarfieldModule {
    pub fn run(
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
        options: StarfieldModuleOptions,
    ) {
        task_tracker.spawn(async move {
            let bernoulli = Bernoulli::new(options.probability);
            let Ok(bernoulli) = bernoulli else {
                eprintln!(
                    "Bernoulli distribution creation gave error: {}",
                    bernoulli.unwrap_err()
                );
                return;
            };
            let mut currently_in_animation: Vec<(u32, f32)> = Vec::new();
            for led in module_leds.iter().flatten() {
                keyboard_controller
                    .set_led_by_index(*led, options.background)
                    .await;
            }

            let mut last_update = Instant::now();
            loop {
                if cancellation_token.is_cancelled() {
                    break;
                }
                if bernoulli.sample(&mut rand::thread_rng())
                    || currently_in_animation.len() < options.min_currently_in_animation as usize
                {
                    let led_start = rand::thread_rng().gen_range(0..module_leds.len() as u32);
                    currently_in_animation.push((led_start, 0.));
                }

                let now = Instant::now();
                for (led, progress) in currently_in_animation.iter_mut() {
                    keyboard_controller
                        .set_led_by_index(
                            *led,
                            animation_curve(options.background, options.target_color, *progress),
                        )
                        .await;
                    *progress +=
                        (now - last_update).as_secs_f32() / options.animation_time.as_secs_f32();
                }

                currently_in_animation.retain(|(_, progress)| *progress <= 1.);

                last_update = Instant::now();

                tokio::time::sleep(Duration::from_millis(10)).await;
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
