use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::bail;

use self::core::args;
use self::core::config_manager::Configuration;
use self::core::keyboard_controller::KeyboardController;
use self::core::module::LinearModule;
use self::core::{config_creator, config_manager};
use self::modules::workspaces::WorkspacesModule;

mod core;
mod modules;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = args::parse_args();
    let keymap_path = match args.keymap_path {
        Some(keymap_path) => Some(PathBuf::from(&keymap_path)),
        None => dirs::config_dir(),
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
            // Create config if it doesn't exist
            if error
                .downcast_ref::<std::io::Error>()
                .map_or(false, |error| error.kind() == io::ErrorKind::NotFound)
            {
                eprintln!("Config not found. Creating it");
                keyboard_controller = KeyboardController::connect(Configuration::default()).await?;
                config_manager::write_config(&keymap_path, &Configuration::default())?;
                let configuration =
                    config_creator::start_config_creator(&keyboard_controller).await?;
                config_manager::write_config(&keymap_path, &configuration)?;
                keyboard_controller.change_configuration(configuration);
            }
            panic!("Error reading file. {}", error)
        }
    };

    keyboard_controller.turn_all_off().await;
    let arc = Arc::new(keyboard_controller);
    WorkspacesModule::run(
        arc.clone(),
        vec![
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        ],
    )
    .await
    .unwrap();
    Ok(())
}
