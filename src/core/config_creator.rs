use std::io::{self, BufRead};

use anyhow::bail;
use crossterm::event::*;
use openrgb::data::Color;

use crate::core::config_manager::Configuration;

use super::keyboard_controller::KeyboardController;
use super::module::{Module, ModuleType};

pub(crate) async fn start_config_creator(
    keyboard_controller: &KeyboardController,
    led_limit: Option<u32>,
) -> anyhow::Result<Configuration> {
    let mut config = Configuration::default();
    println!("Press every key as it lights up. If no key lights up, press LMB. If no reaction is given when key is pressed, press RMB");
    build_key_led_map(keyboard_controller, &mut config, led_limit).await?;
    println!("Press the first keys of every row. In order. When all of them have been pressed, press LMB");
    build_first_in_row(keyboard_controller, &mut config).await?;
    println!("Now, we're going to place the modules. First of all, choose a module to add:");
    let module_type = choose_module_to_add()?;
    println!("Click the buttons which are in this module IN ORDER from left to right. Press LMB when done. Press RMB to add a button to the module which is not tied to any LED");
    add_module(keyboard_controller, &mut config, module_type).await?;
    Ok(config)
}

async fn add_module(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
    module_type: ModuleType,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
    let mut module_leds = Vec::new();
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
            default_terminal_settings()?;
            if let Some(&index_pressed) = config.key_led_map.get(&event.code) {
                module_leds.push(Some(index_pressed));
                keyboard_controller
                    .set_led_by_index(index_pressed, Color::new(255, 255, 255))
                    .await?;
            } else {
                println!("This button is not tied to an LED, it can't be used in module");
            }
            prepare_terminal_event_capture()?;
        }
        if let Event::Mouse(event) = event {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                default_terminal_settings()?;
                println!("Creating module {}", module_type.name());
                let module = Module::new(module_type, module_leds);
                config.modules.push(module);
                break;
            }
            if event.kind == MouseEventKind::Down(MouseButton::Right) {
                module_leds.push(None);
            }
        }
    }
    default_terminal_settings()?;
    Ok(())
}

fn choose_module_to_add() -> anyhow::Result<ModuleType> {
    let all_module_types = ModuleType::all_module_types();
    for (i, module) in all_module_types.iter().enumerate() {
        println!("{}) {} -- {}", i + 1, module.name(), module.desc())
    }
    default_terminal_settings()?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(number_chosen) = line?.parse::<usize>() else {
            println!("Not a number");
            continue;
        };
        if number_chosen > all_module_types.len() {
            println!("Not an option");
            continue;
        }
        return Ok(all_module_types[number_chosen - 1]);
    }
    bail!("No option chosen");
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
fn prepare_terminal_event_capture() -> anyhow::Result<()> {
    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;

    if supports_keyboard_enhancement {
        crossterm::queue!(
            stdout,
            PushKeyboardEnhancementFlags(
                // KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                // | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                 | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            )
        )
        .unwrap();
    }
    crossterm::execute!(
        stdout,
        // EnableBracketedPaste,
        // EnableFocusChange,
        EnableMouseCapture,
    )
    .unwrap();
    Ok(())
}

fn default_terminal_settings() -> anyhow::Result<()> {
    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );
    let mut stdout = io::stdout();
    crossterm::terminal::disable_raw_mode()?;

    if supports_keyboard_enhancement {
        crossterm::queue!(stdout, PopKeyboardEnhancementFlags).unwrap();
    }
    crossterm::execute!(
        stdout,
        // EnableBracketedPaste,
        // EnableFocusChange,
        DisableMouseCapture,
    )
    .unwrap();
    Ok(())
}
