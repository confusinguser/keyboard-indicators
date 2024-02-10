use std::io::{self, BufRead};
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::bail;
use clap::ArgMatches;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use openrgb::data::Color;
use tokio::sync::mpsc::{self, Sender};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::config_manager::{self, Configuration};
use crate::core::keyboard_controller::KeyboardController;
use crate::core::module::{Module, ModuleType};
use crate::core::utils::{
    self, default_terminal_settings, highlight_all_modules, highlight_one_module,
    highlight_one_module_rainbow, prepare_terminal_event_capture,
};

pub async fn module(args: &ArgMatches) -> anyhow::Result<()> {
    let config_path = utils::get_config_path(args)?;
    let keymap_path = utils::get_keymap_path(args)?;
    let mut config = config_manager::read_config_and_keymap(&config_path, &keymap_path)?;
    let keyboard_controller = KeyboardController::connect().await?;
    let (mut sender, receiver) = mpsc::channel(100);
    let cancellation_token = CancellationToken::new();
    let keyboard_controller = Arc::new(Mutex::new(keyboard_controller));
    KeyboardController::run(
        keyboard_controller.clone(),
        &TaskTracker::new(),
        cancellation_token,
        receiver,
    );
    keyboard_controller.lock().await.turn_all_off().await?;

    match args.subcommand_name() {
        Some("add") => add(&mut sender, &mut config).await?,
        Some("remove") => remove(keyboard_controller, &mut sender, &mut config).await?,
        Some("info") => info(&mut sender, &mut config).await?,
        Some("modify") => todo!(),
        _ => todo!(),
    }

    config_manager::write_config_and_keymap(&config_path, &keymap_path, &config)?;

    Ok(())
}

pub async fn add(
    sender: &mut Sender<(u32, Color)>,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    println!("Choose a module to add:");
    let module_type = choose_module_type_to_add()?;
    println!("Click the buttons which are in this module IN ORDER from left to right. Press LMB when done. Press RMB to add a button to the module which is not tied to any LED");
    add_module(sender, config, module_type).await?;

    Ok(())
}

async fn add_module(
    sender: &mut Sender<(u32, Color)>,
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
            if let Some(&index_pressed) = config.keymap.key_led_map.get(&event.code) {
                if module_leds.contains(&Some(index_pressed)) {
                    println!("This LED has already been added");
                    prepare_terminal_event_capture()?;
                    continue;
                }
                module_leds.push(Some(index_pressed));
                KeyboardController::update_led(sender, index_pressed, Color::new(255, 255, 255))
                    .await?;
            } else {
                println!("This button is not tied to an LED, it can't be used in module");
            }
            prepare_terminal_event_capture()?;
        }
        if let Event::Mouse(event) = event {
            if event.kind == MouseEventKind::Down(MouseButton::Left) {
                default_terminal_settings()?;
                println!("Creating {}", module_type.name());
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
    // TODO, make this work
    // KeyboardController::turn_all_off().await?;
    Ok(())
}

fn choose_module_type_to_add() -> anyhow::Result<ModuleType> {
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

pub async fn remove(
    keyboard_controller: Arc<Mutex<KeyboardController>>,
    sender: &mut Sender<(u32, Color)>,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    if config.modules.is_empty() {
        println!("There are no modules to remove");
        return Ok(());
    }

    println!("Choose a module to remove by clicking on a button in it");
    highlight_all_modules(sender, config, 100., 100.).await?;

    utils::prepare_terminal_event_capture()?;
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
            if let Some(&index_pressed) = config.keymap.key_led_map.get(&event.code) {
                let mut module_selected = None;
                for (i, module) in config.modules.iter().enumerate() {
                    for led in &module.module_leds {
                        let Some(led) = led else {
                            continue;
                        };
                        if index_pressed == *led {
                            module_selected = Some((i, module));
                            break;
                        }
                    }
                    if module_selected.is_some() {
                        break;
                    }
                }
                if let Some(module_selected) = module_selected {
                    keyboard_controller.lock().await.turn_all_off().await?;
                    highlight_one_module(
                        sender,
                        config.modules.len(),
                        module_selected.0,
                        module_selected.1,
                    )
                    .await?;

                    print_module_info(module_selected.1);
                    let module_removal_confirmed = utils::confirm_action(
                        "Are you sure you want to remove this module? [y/N] ",
                        false,
                    )?;

                    if module_removal_confirmed {
                        config.modules.remove(module_selected.0);
                    }
                    keyboard_controller.lock().await.turn_all_off().await?;
                    break;
                } else {
                    println!("This button is not tied to a module");
                }
            } else {
                println!("This button is not tied to an LED");
            }
            prepare_terminal_event_capture()?;
        }
    }
    default_terminal_settings()?;
    Ok(())
}

fn print_module_info(module: &Module) {
    println!(
        "Module: {}
Description: {}
Number of LEDs: {}",
        module.module_type.name(),
        module.module_type.desc(),
        module.module_leds.len()
    );
}

async fn info(sender: &mut Sender<(u32, Color)>, config: &mut Configuration) -> anyhow::Result<()> {
    println!("Choose a module to get info on by clicking a button in it");
    highlight_all_modules(sender, config, 100., 100.).await?;

    utils::prepare_terminal_event_capture()?;
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
            if let Some(&index_pressed) = config.keymap.key_led_map.get(&event.code) {
                let mut module_selected = None;
                for module in &config.modules {
                    for led in &module.module_leds {
                        let Some(led) = led else {
                            continue;
                        };
                        if index_pressed == *led {
                            module_selected = Some(module);
                            break;
                        }
                    }
                    if module_selected.is_some() {
                        break;
                    }
                }
                if let Some(module_selected) = module_selected {
                    // TODO Make user select between each module that has this button, also for remove
                    // subcommand
                    print_module_info(module_selected);
                    //TODO
                    // keyboard_controller.turn_all_off().await?;
                    highlight_all_modules(sender, config, 80., 10.).await?;
                    highlight_one_module_rainbow(sender, module_selected).await?;
                    utils::pause_until_click()?;

                    // keyboard_controller.turn_all_off().await?;
                    break;
                } else {
                    println!("This button is not tied to a module");
                }
            } else {
                println!("This button is not tied to an LED");
            }
            prepare_terminal_event_capture()?;
        }
    }
    default_terminal_settings()?;
    Ok(())
}
