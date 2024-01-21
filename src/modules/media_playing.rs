use rgb::{ComponentMap, RGB8};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::core::keyboard_controller::KeyboardController;
use crate::core::{constants, utils};

pub(crate) struct MediaModule {}

impl MediaModule {
    pub(crate) fn run(
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
    ) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();
        let track_duration: Arc<Mutex<Option<Duration>>> = Arc::new(Mutex::new(None));
        let track_duration_clone = track_duration.clone();
        let metadata_listener = tokio::spawn(async {
            let track_duration = track_duration_clone;
            let stdout = utils::run_command_async("playerctl metadata -F")
                .expect("run_command_async returned None in metadata_listener");
            let mut buffer = BufReader::new(stdout).lines();
            loop {
                let Ok(Some(metadata_line)) = buffer
                    .next_line()
                    .await
                    .map_err(|err| eprintln!("Failed to get next line of metadata. {}", err))
                else {
                    continue;
                };
                if metadata_line.contains("length") {
                    let Some(Ok(length)) = utils::run_command("playerctl metadata mpris:length")
                        .map(|length| length.parse::<u64>())
                    else {
                        // No player available
                        *track_duration
                            .lock()
                            .expect("Acquire track_duration Mutex lock") = None;
                        continue;
                    };
                    *track_duration
                        .lock()
                        .expect("Acquire track_duration Mutex lock") =
                        Some(Duration::from_micros(length))
                }
            }
        });
        handles.push(metadata_listener);
        let handle = tokio::spawn(async move {
            let mut paused_since_last_time = false;
            let mut last_progress = None;
            loop {
                let (color, is_paused) = get_module_color_and_status();
                if is_paused {
                    if !paused_since_last_time {
                        paused_since_last_time = true;
                        last_progress = None;
                        let keyboard_controller_clone = keyboard_controller.clone();

                        let module_leds_clone = module_leds.clone();
                        // flatten() filters out None
                        for led_index in module_leds_clone.into_iter().flatten() {
                            keyboard_controller_clone
                                .set_led_by_index(led_index, color)
                                .await;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                paused_since_last_time = false;

                let Some(current_position) = utils::run_command("playerctl position") else {
                    eprintln!("Current position output is None");
                    continue;
                };
                let Ok(current_position) = current_position.parse::<f32>() else {
                    eprintln!(
                        "Current position output is not a number. It is: {}",
                        current_position
                    );
                    continue;
                };
                let current_position = Duration::from_micros((current_position * 1e6_f32) as u64);

                let progress;
                {
                    let track_duration_value = track_duration
                        .lock()
                        .expect("Acquire track_duration Mutex lock");

                    progress = if track_duration_value.is_some() {
                        (current_position.as_millis() as f64
                            / track_duration_value.unwrap().as_millis() as f64)
                            as f32
                    } else {
                        1.
                    };
                }

                for order in utils::progress_bar_diff(
                    progress,
                    last_progress,
                    module_leds.len() as u32,
                    false,
                ) {
                    let keyboard_controller_clone = keyboard_controller.clone();
                    let led_index = module_leds[order.0 as usize];
                    if let Some(led_index) = led_index {
                        keyboard_controller_clone
                            .set_led_by_index(
                                led_index,
                                color.map(|comp| (comp as f32 * order.1) as u8),
                            )
                            .await;
                    }
                }
                last_progress = Some(progress);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        handles.push(handle);
        handles
    }
}

/// The second element denotes if all players are paused
fn get_module_color_and_status() -> (RGB8, bool) {
    let mut playing_platforms = Vec::new();
    for (i, platform) in utils::run_command("playerctl status -a")
        .unwrap()
        .split('\n')
        .enumerate()
    {
        if platform == "Playing" {
            playing_platforms.push(i);
        }
    }

    if playing_platforms.is_empty() {
        return (constants::PAUSED_MEDIA_PLAYING_COLOR, true);
    }

    for (i, platform) in utils::run_command("playerctl -l")
        .unwrap()
        .split('\n')
        .enumerate()
    {
        if platform == "spotify" && playing_platforms.binary_search(&i).is_ok() {
            // Spotify is playing
            return (constants::SPOTIFY_MEDIA_PLAYING_COLOR, false);
        }
    }

    for (i, metadata_line) in utils::run_command("playerctl metadata xesam:title -a")
        .unwrap()
        .split('\n')
        .enumerate()
    {
        if metadata_line.to_lowercase().contains("netflix")
            && playing_platforms.binary_search(&i).is_ok()
        {
            return (constants::NETFLIX_MEDIA_PLAYING_COLOR, false);
        }
    }

    (constants::DEFAULT_MEDIA_PLAYING_COLOR, false)
}