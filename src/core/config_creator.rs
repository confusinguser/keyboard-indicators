use crossterm::event::*;
use openrgb::data::Color;

use crate::cli::module_subcommand;
use crate::core::config_manager::Configuration;
use crate::core::utils::default_terminal_settings;

use super::keyboard_controller::KeyboardController;
use super::utils::prepare_terminal_event_capture;

pub(crate) async fn start_config_creator(
    keyboard_controller: &KeyboardController,
    led_limit: Option<u32>,
) -> anyhow::Result<Configuration> {
    let mut config = Configuration::default();
    println!("Press every key as it lights up. If no key lights up, press LMB. If no reaction is given when key is pressed, press RMB");
    build_key_led_map(keyboard_controller, &mut config, led_limit).await?;
    println!("Press the first keys of every row. In order. When all of them have been pressed, press LMB");
    build_first_in_row(keyboard_controller, &mut config).await?;
    println!("Now, we're going to place the modules");
    module_subcommand::add(keyboard_controller, &mut config).await?;
    Ok(config)
}

async fn build_first_in_row(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
    keyboard_controller.turn_all_off().await?;
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
            if let Some(&index_pressed) = config.key_led_map.get(&event.code) {
                config.first_in_row.push(index_pressed);
                keyboard_controller
                    .set_led_by_index(index_pressed, Color::new(255, 255, 255))
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
                println!("Great. There are {} rows", config.first_in_row.len());
                keyboard_controller.turn_all_off().await?;
                break;
            }
        }
    }
    default_terminal_settings()?;
    Ok(())
}

async fn build_key_led_map(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
    led_limit: Option<u32>,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
    keyboard_controller.turn_all_off().await?;
    for index in 0..keyboard_controller
        .num_leds()
        .await
        .min(led_limit.unwrap_or(u32::MAX))
    {
        if index != 0 {
            keyboard_controller
                .set_led_by_index(index - 1, Color::new(0, 0, 0))
                .await?;
        }
        keyboard_controller
            .set_led_by_index(index, Color::new(255, 255, 255))
            .await?;
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
                    config.key_led_map.insert(event.code, index);
                    break;
                }

                Event::Mouse(event) => match event.kind {
                    MouseEventKind::Down(mouse_button) => {
                        if mouse_button == MouseButton::Left {
                            config.skip_indicies.insert(index);
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
