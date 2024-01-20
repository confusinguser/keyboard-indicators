use std::io;
use std::path::Path;
use std::sync::Arc;

use self::core::config_manager::Configuration;
use self::core::keyboard_controller::KeyboardController;
use self::core::module::LinearModule;
use self::core::{config_creator, config_manager};
use self::modules::media_playing::MediaModule;
use self::modules::workspaces::WorkspacesModule;

mod core;
mod modules;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = Path::new("config.yaml");
    let mut keyboard_controller;
    match config_manager::read_config(config_path) {
        Ok(configuration) => {
            keyboard_controller = KeyboardController::connect(configuration).await?;
        }
        Err(error) => {
            if error
                .downcast_ref::<std::io::Error>()
                .map_or(false, |error| error.kind() == io::ErrorKind::NotFound)
            {
                eprintln!("Config not found. Creating it");
                keyboard_controller = KeyboardController::connect(Configuration::default()).await?;
                let configuration =
                    config_creator::start_config_creator(&keyboard_controller).await?;
                config_manager::write_config(config_path, &configuration)?;
                keyboard_controller.change_configuration(configuration);
            }
            panic!("Error reading file. {}", error)
        }
    };

    // keyboard_controller.turn_all_off().await;
    let arc = Arc::new(keyboard_controller);
    WorkspacesModule::run(
        arc.clone(),
        vec![24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 3, 4, 5, 6],
    );
    MediaModule::run(arc.clone(), vec![7, 8, 9, 10])
        .await
        .unwrap();
    Ok(())
}
