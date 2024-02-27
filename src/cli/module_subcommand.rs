use anyhow::bail;
use std::sync::Arc;
use tokio::sync::Mutex;

use clap::ArgMatches;
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use openrgb::data::Color;
use tokio::sync::mpsc::{self, Sender};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::config_manager::{self, Configuration};
use crate::core::keyboard_controller::{KeyboardController, KeyboardControllerMessage};
use crate::core::module::{Module, ModuleType};
use crate::core::utils::{
    self, default_terminal_settings, highlight_all_modules, highlight_one_module,
    highlight_one_module_rainbow, prepare_terminal_event_capture, rgb_to_hex,
};
use crate::modules::noise::NoiseModuleOptions;
use crate::modules::starfield::StarfieldModuleOptions;

pub async fn module(args: &ArgMatches) -> anyhow::Result<()> {
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

    match args.subcommand_name() {
        Some("add") => add(&mut sender, &mut config).await?,
        Some("remove") => remove(&mut sender, &mut config).await?,
        Some("info") => info(&mut sender, &config).await?,
        Some("modify") => modify(&mut sender, &mut config).await?,
        _ => todo!(),
    }

    config_manager::write_config_and_keymap(&config.config_path, &config.keymap_path, &config)?;

    Ok(())
}

pub async fn add(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    println!("Choose a module to add:");
    let module_type = choose_module_type_to_add()?;
    println!("Click the buttons which are in this module IN ORDER from left to right. Press LMB when done. Press RMB to add a button to the module which is not tied to any LED");
    add_module(sender, config, module_type).await?;

    Ok(())
}

async fn add_module(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &mut Configuration,
    module_type: ModuleType,
) -> anyhow::Result<()> {
    prepare_terminal_event_capture()?;
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
    KeyboardController::turn_all_off(sender).await?;
    Ok(())
}

fn choose_module_type_to_add() -> anyhow::Result<ModuleType> {
    let all_module_types = ModuleType::all_module_types();
    let options: Vec<String> = all_module_types
        .iter()
        .map(|module| format!("{} -- {}", module.name(), module.desc()))
        .collect();

    Ok(all_module_types[utils::choose_option(&options)?])
}

/// TODO make lock for this. If multiple processes run with this, it can lead to bad stuff
pub async fn remove(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    if config.modules.is_empty() {
        println!("There are no modules to remove");
        return Ok(());
    }

    println!("Choose a module to remove by clicking on a button in it");
    let module_index = choose_module_on_keyboard(sender, config).await?;
    let Some(module) = config.modules.get(module_index) else {
        bail!("No module selected")
    };
    KeyboardController::turn_all_off(sender).await?;

    highlight_one_module(sender, config.modules.len(), module_index, module).await?;

    print_module_info(module);
    let module_removal_confirmed =
        utils::confirm_action("Are you sure you want to remove this module? [y/N] ", false)?;

    if module_removal_confirmed {
        config.modules.remove(module_index);
    }
    KeyboardController::turn_all_off(sender).await?;

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

async fn choose_module_on_keyboard(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &Configuration,
) -> anyhow::Result<usize> {
    highlight_all_modules(sender, config, 100., 100.).await?;
    prepare_terminal_event_capture()?;

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
                let mut module_index = None;
                for (i, module) in config.modules.iter().enumerate() {
                    for led in module.module_leds.iter().flatten() {
                        if index_pressed == *led {
                            module_index = Some(i);
                            break;
                        }
                    }
                    if module_index.is_some() {
                        break;
                    }
                }
                let Some(module_index) = module_index else {
                    println!("This button is not tied to a module");
                    continue;
                };
                // TODO Make user select between each module that has this button
                return Ok(module_index);
            } else {
                println!("This button is not tied to an LED");
            }
            prepare_terminal_event_capture()?;
        }
    }
}

async fn info(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &Configuration,
) -> anyhow::Result<()> {
    println!("Choose a module to get info on by clicking a button in it");
    let module_index = choose_module_on_keyboard(sender, config).await?;
    let module = config.modules.get(module_index).unwrap();

    print_module_info(module);
    KeyboardController::turn_all_off(sender).await?;
    highlight_all_modules(sender, config, 80., 10.).await?;
    highlight_one_module_rainbow(sender, module).await?;
    utils::pause_until_click()?;

    KeyboardController::turn_all_off(sender).await?;
    default_terminal_settings()?;
    Ok(())
}

async fn modify(
    sender: &mut Sender<KeyboardControllerMessage>,
    config: &mut Configuration,
) -> anyhow::Result<()> {
    let module_index = choose_module_on_keyboard(sender, config).await?;

    // Present options for modification and handle user input
    loop {
        let Some(module) = config.modules.get_mut(module_index) else {
            bail!("No module selected")
        };
        let option = utils::choose_option(&[
            "Modify LEDs",
            "Modify settings",
            "Reset settings to default",
            "Exit",
        ])?;

        match option {
            0 => modify_leds(module)?,
            1 => modify_settings(module)?,
            2 => reset_settings_to_default(module),
            3 => break,
            _ => println!("Invalid option"),
        }

        // Save after each change. Investigate if this is better than having a save and exit TODO
        config_manager::write_config_and_keymap(&config.config_path, &config.keymap_path, config)?;
    }

    Ok(())
}

fn reset_settings_to_default(module: &mut Module) {
    match module.module_type {
        ModuleType::Workspaces => {}
        ModuleType::Media => {}
        ModuleType::Starfield(ref mut opts) => *opts = StarfieldModuleOptions::default(),
        ModuleType::Noise(ref mut opts) => *opts = NoiseModuleOptions::default(),
    }
    println!("Reset settings to default")
}

fn modify_settings(module: &mut Module) -> anyhow::Result<()> {
    macro_rules! add_choice {
        ($choices: tt, $val: expr, $name: expr) => {
            $choices.push(format!("{} [Current: {}]", $name, $val));
        };
    }
    let mut choices = Vec::new();
    match module.module_type {
        ModuleType::Workspaces => todo!(),
        ModuleType::Media => todo!(),
        ModuleType::Starfield(opts) => {
            todo!()
        }
        ModuleType::Noise(ref mut opts) => {
            add_choice!(choices, rgb_to_hex(opts.color1), "First color");
            add_choice!(choices, rgb_to_hex(opts.color2), "Second color");
            add_choice!(choices, opts.speed, "Speed");
            add_choice!(choices, opts.zoom_factor, "Zoom factor");
            let option_chosen = utils::choose_option(&choices)?;

            println!("Enter new value: ");
            match option_chosen {
                0 => {
                    let new_color = utils::get_color_input()?;
                    opts.color1 = new_color;
                }
                1 => {
                    let new_color = utils::get_color_input()?;
                    opts.color2 = new_color;
                }
                2 => {
                    let new_speed =
                        utils::get_input("Invalid number", |input| input.parse::<f32>().ok())?;
                    opts.speed = new_speed;
                }
                3 => {
                    let new_zoom_factor =
                        utils::get_input("Invalid number", |input| input.parse::<f32>().ok())?;
                    opts.zoom_factor = new_zoom_factor;
                }
                _ => {}
            };
        }
    }

    Ok(())
}

fn modify_leds(module_to_modify: &mut Module) -> anyhow::Result<()> {
    // Implement logic to modify LEDs of the module
    Ok(())
}
