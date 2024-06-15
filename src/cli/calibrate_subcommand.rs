use std::sync::Arc;

use clap::ArgMatches;
use openrgb::data::Color;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::{config_manager, utils};
use crate::core::keyboard_controller::KeyboardController;

pub async fn calibrate(args: &ArgMatches) -> anyhow::Result<()> {
    let config_path = utils::get_config_path(args)?;
    let keymap_path = utils::get_keymap_path(args)?;
    let mut config = config_manager::read_config_and_keymap(config_path, keymap_path)?;
    let keyboard_controller = KeyboardController::connect().await?;
    let (mut sender, receiver) = mpsc::channel(200);
    let cancellation_token = CancellationToken::new();
    let keyboard_controller = Arc::new(Mutex::new(keyboard_controller));
    KeyboardController::run(
        keyboard_controller.clone(),
        &TaskTracker::new(),
        cancellation_token,
        receiver,
    );
    KeyboardController::turn_all_off(&mut sender).await?;

    println!("Press a long line of keys, one at a time. Press LMB when done");
    let leds_picked = utils::pick_leds(
        &mut sender,
        &config.keymap.key_led_map,
        Color::new(255, 255, 255),
    )
        .await?;
    println!("Pick the LED which has the brightness in between the turned off LEDs and the brightest LED");
    for (i, led) in leds_picked.iter().enumerate() {
        if let Some(&led) = led.as_ref() {
            KeyboardController::update_led(
                &mut sender,
                led,
                Color::new(
                    (255. / leds_picked.len() as f32 * (i as f32 + 1.)) as u8,
                    0,
                    0,
                ),
            )
                .await?;
        }
    }
    let mut middle_led_index = None;
    while middle_led_index.is_none() {
        let middle_led = utils::pick_led(&config.keymap.key_led_map).await?;
        middle_led_index = leds_picked.iter().position(|led| led == &Some(middle_led));
        if middle_led_index.is_none() {
            println!("Not one of the bright LEDs");
        }
    }
    let middle_led_index = middle_led_index.unwrap();
    let middle_led_brightness =
        (255. / leds_picked.len() as f32 * (middle_led_index as f32 + 1.)) as u8;
    let utils::calculate_k_value(middle_led_brightness);

    config_manager::write_config_and_keymap(&config.config_path, &config.keymap_path, &config)?;

    Ok(())
}
