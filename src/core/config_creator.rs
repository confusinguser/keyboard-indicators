use std::sync::Arc;
use tokio::sync::Mutex;

use crossterm::event::*;
use openrgb::data::Color;
use tokio::sync::mpsc::Sender;

use crate::cli::module_subcommand;
use crate::core::config_manager::Configuration;
use crate::core::keymap::Keymap;
use crate::core::utils::default_terminal_settings;

use super::keyboard_controller::KeyboardController;
use super::utils::prepare_terminal_event_capture;

pub(crate) async fn start_config_creator(
    keyboard_controller: Arc<Mutex<KeyboardController>>,
    sender: &mut Sender<(u32, Color)>,
    led_limit: Option<u32>,
) -> anyhow::Result<Configuration> {
    let mut config = Configuration::default();
    println!("Press every key as it lights up. If no key lights up, press LMB. If no reaction is given when key is pressed, press RMB");
    build_key_led_map(keyboard_controller, sender, &mut config.keymap, led_limit).await?;
    println!(
        "Press the first key of every row. In order. When all of them have been pressed, press LMB"
    );
    build_first_in_row(sender, &mut config.keymap).await?;
    println!("Now, we're going to place the modules");
    module_subcommand::add(sender, &mut config).await?;
    Ok(config)
}

async fn build_first_in_row(
    sender: &mut Sender<(u32, Color)>,
    keymap: &mut Keymap,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
    //TODO
    // keyboard_controller.turn_all_off().await?;
    loop {
        let event = crossterm::event::read().unwrap();
        if let Event::Key(event) = event {
            if event.kind != KeyEventKind::Press {
                continue;
            }
            if event.modifiers.intersects(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c')
            {
                default_terminal_settings()?;
                panic!("Interrupted by user");
            }
            if let Some(&index_pressed) = keymap.key_led_map.get(&event.code) {
                keymap.first_in_row.push(index_pressed);
                KeyboardController::update_led(sender, index_pressed, Color::new(255, 255, 255))
                    .await?;
            } else {
                default_terminal_settings()?;
                println!("This button was not pressed in the last stage, it can't be marked as first button in row");
                prepare_terminal_event_capture()?;
            }
        }
        if let Event::Mouse(event) = event {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                default_terminal_settings()?;
                println!("Great. There are {} rows", keymap.first_in_row.len());
                // keyboard_controller.turn_all_off().await?;
                break;
            }
        }
    }
    default_terminal_settings()?;
    Ok(())
}

async fn build_key_led_map(
    keyboard_controller: Arc<Mutex<KeyboardController>>,
    sender: &mut Sender<(u32, Color)>,
    keymap: &mut Keymap,
    led_limit: Option<u32>,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
    keyboard_controller.lock().await.turn_all_off().await?;
    for index in 0..keyboard_controller
        .lock()
        .await
        .num_leds()
        .min(led_limit.unwrap_or(u32::MAX))
    {
        if index != 0 {
            KeyboardController::update_led(sender, index - 1, Color::new(0, 0, 0)).await?;
        }
        KeyboardController::update_led(sender, index, Color::new(255, 255, 255)).await?;
        loop {
            let event = crossterm::event::read().unwrap();
            match event {
                Event::Key(event) => {
                    if event.kind != KeyEventKind::Press {
                        continue;
                    }
                    if event.modifiers.intersects(KeyModifiers::CONTROL)
                        && event.code == KeyCode::Char('c')
                    {
                        default_terminal_settings()?;
                        panic!("Interrupted by user");
                    }
                    keymap.key_led_map.insert(event.code, index);
                    break;
                }

                Event::Mouse(event) => match event.kind {
                    MouseEventKind::Down(mouse_button) => {
                        if mouse_button == MouseButton::Left {
                            keymap.skip_indicies.insert(index);
                        }
                        break;
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
    }
    default_terminal_settings()?;
    Ok(())
}

pub(crate) async fn create_keymap(
    keyboard_controller: Arc<Mutex<KeyboardController>>,
    sender: &mut Sender<(u32, Color)>,
    led_limit: Option<u32>,
) -> anyhow::Result<Keymap> {
    let mut keymap = Keymap::default();
    println!("Press every key as it lights up. If no key lights up, press LMB. If no reaction is given when key is pressed, press RMB");
    build_key_led_map(keyboard_controller, sender, &mut keymap, led_limit).await?;
    println!(
        "Press the first key of every row. In order. When all of them have been pressed, press LMB"
    );
    build_first_in_row(sender, &mut keymap).await?;
    Ok(keymap)
}
