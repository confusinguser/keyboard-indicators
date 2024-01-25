use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use rgb::RGB8;

use self::core::cli::{self, Cli, CreateConfigArgs, StartArgs};
use self::core::config_manager::Configuration;
use self::core::keyboard_controller::KeyboardController;
use self::core::{config_creator, config_manager};
use self::modules::starfield::{StarfieldModule, StarfieldModuleOptions};

mod core;
mod modules;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::parse_args();
    match args {
        Cli::Start(start_args) => start(start_args).await,
        Cli::CreateConfig(create_config_args) => create_config(create_config_args).await,
    }
}

async fn create_config(args: CreateConfigArgs) -> anyhow::Result<()> {
    let keymap_path = match args.keymap_path {
        Some(keymap_path) => Some(PathBuf::from(&keymap_path)),
        None => dirs::config_dir().map(|pathbuf| pathbuf.join("keyboard-indicators/keymap.yaml")),
    };

    let Some(keymap_path) = keymap_path else {
        bail!("Could not find a path for config. Please specify your own");
    };
    let keyboard_controller = KeyboardController::connect(Configuration::default()).await?;
    config_manager::write_config(&keymap_path, &Configuration::default())?;
    let new_config = config_creator::start_config_creator(&keyboard_controller).await?;
    config_manager::write_config(&keymap_path, &new_config)?;
    Ok(())
}

async fn start(args: StartArgs) -> anyhow::Result<()> {
    let keymap_path = match args.keymap_path {
        Some(keymap_path) => Some(PathBuf::from(&keymap_path)),
        None => dirs::config_dir().map(|pathbuf| pathbuf.join("keyboard-indicators/keymap.yaml")),
    };

    let Some(keymap_path) = keymap_path else {
        bail!("Could not find a path for config. Please specify your own");
    };

    let mut keyboard_controller;
    match config_manager::read_config(&keymap_path) {
        Ok(configuration) => {
            keyboard_controller = KeyboardController::connect(configuration).await?;
        }
        Err(error) => {
            let mut recreate_config = false;
            // Create config if it doesn't exist
            if error
                .downcast_ref::<std::io::Error>()
                .map_or(false, |error| error.kind() == io::ErrorKind::NotFound)
            {
                eprintln!("Config not found. Creating it in {:?}", keymap_path);
                recreate_config = true;
            }
            if error.downcast_ref::<serde_yaml::Error>().is_some() {
                println!("Error with deserializing. Recreating it.");
                recreate_config = true;
            }
            if recreate_config {
                keyboard_controller = KeyboardController::connect(Configuration::default()).await?;
                config_manager::write_config(&keymap_path, &Configuration::default())?;
                let new_config = config_creator::start_config_creator(&keyboard_controller).await?;
                config_manager::write_config(&keymap_path, &new_config)?;
                keyboard_controller.config = new_config;
            } else {
                panic!("Error reading file {:?}. {}", keymap_path, error)
            }
        }
    };

    keyboard_controller.turn_all_off().await?;
    let keyboard_controller = Arc::new(keyboard_controller);
    let mut join_hooks = Vec::new();
    for module in &keyboard_controller.config.modules {
        join_hooks.append(
            &mut module
                .module_type
                .run(keyboard_controller.clone(), module.module_leds.clone()),
        );
    }

    let mut vec = StarfieldModule::run(
        keyboard_controller.clone(),
        vec![
            Some(24),
            Some(25),
            Some(26),
            Some(27),
            Some(28),
            Some(29),
            Some(30),
            Some(31),
            Some(32),
            Some(33),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
            Some(9),
            Some(10),
            Some(11),
            Some(12),
            Some(13),
            Some(14),
        ],
        StarfieldModuleOptions {
            background: RGB8::new(0xF7, 0xCA, 0x18),
            min_currently_in_animation: 5,
            target_color: RGB8::new(0x00, 128, 0),
            probability: 0.0004,
            animation_time: Duration::from_secs(1),
        },
    );

    join_hooks.append(&mut vec);
    // Make sure to not exit if threads are open
    for hook in join_hooks {
        hook.await?;
    }
    Ok(())
}
