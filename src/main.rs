use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use rgb::RGB8;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

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

    let cancellation_token = CancellationToken::new();
    keyboard_controller.turn_all_off().await?;
    let keyboard_controller = Arc::new(keyboard_controller);
    let task_tracker = TaskTracker::new();
    for module in &keyboard_controller.config.modules {
        module.module_type.run(
            &task_tracker,
            cancellation_token.clone(),
            keyboard_controller.clone(),
            module.module_leds.clone(),
        );
    }

    task_tracker.close();

    match signal::ctrl_c().await {
        Ok(_) => {
            println!("Ctrl C received");
            cancellation_token.cancel();
        }
        Err(_) => {
            println!("Cannot receive Ctrl C signals, shutting down");
            cancellation_token.cancel();
        }
    }

    // Make sure to not exit if threads are open
    task_tracker.wait().await;
    Ok(())
}
