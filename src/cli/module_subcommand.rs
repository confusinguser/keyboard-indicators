use std::io::{self, BufRead};
use std::path::PathBuf;

use anyhow::bail;
use clap::ArgMatches;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use openrgb::data::Color;

use crate::core::config_manager::{self, Configuration};
use crate::core::keyboard_controller::KeyboardController;
use crate::core::module::{Module, ModuleType};
use crate::core::utils::{self, default_terminal_settings, prepare_terminal_event_capture};

pub async fn module_subcommand(args: &ArgMatches) -> anyhow::Result<()> {
    let mut config = config_manager::read_config(&utils::get_config_path(args)?)?;
    let keyboard_controller = KeyboardController::connect(Configuration::default()).await?;

    match args.subcommand_name() {
        Some("add") => add(&keyboard_controller, &mut config).await?,
        Some("remove") => todo!(),
        Some("info") => todo!(),
        Some("modify") => todo!(),
        _ => todo!(),
    }

    Ok(())
}

pub async fn add(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    println!("Choose a module to add:");
    let module_type = choose_module_to_add()?;
    println!("Click the buttons which are in this module IN ORDER from left to right. Press LMB when done. Press RMB to add a button to the module which is not tied to any LED");
    add_module(keyboard_controller, config, module_type).await?;

    Ok(())
}

async fn add_module(
    keyboard_controller: &KeyboardController,
    config: &mut Configuration,
    module_type: ModuleType,
) -> anyhow::Result<()> {
    utils::prepare_terminal_event_capture()?;
    let mut module_leds = Vec::new();
    loop {
        let event = crossterm::event::read().unwrap();
        if let Event::Key(event) = event {
            if event.kind != crossterm::event::KeyEventKind::Press {
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
