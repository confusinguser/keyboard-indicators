use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::bail;

use self::core::args;
use self::core::config_manager::Configuration;
use self::core::keyboard_controller::KeyboardController;
use self::core::{config_creator, config_manager};

mod core;
mod modules;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = args::parse_args();
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
    // WorkspacesModule::run(
    //     arc.clone(),
    //     vec![
    //         24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    //     ],
    // )
    // .await
    // .unwrap();
    Ok(())
}
