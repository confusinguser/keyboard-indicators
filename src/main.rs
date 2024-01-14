use std::path::Path;
use std::sync::Arc;

use crate::core::utils;

use self::core::keyboard_controller::KeyboardController;
use self::core::module;
use self::core::module::LinearModule;
use self::core::{config_creator, config_manager};
use self::modules::workspaces::{self, WorkspacesModule};

mod core;
mod modules;
#[tokio::main]
async fn main() {
    let config_path = Path::new("config.yaml");
    let configuration = config_manager::read_config(config_path);
    dbg!(utils::run_command("swaymsg exec kitty"));
    if let Err(error) = &configuration {
        println!("{}. Recreating config.", error)
    }
    let configuration = configuration.unwrap_or_default();
    let keyboard_controller = KeyboardController::connect(configuration).await.unwrap();
    keyboard_controller.turn_all_off().await;
    // let config = config_creator::start_config_creator(keyboard_controller).await;
    let workspaces = workspaces::WorkspacesModule {};
    let arc = Arc::new(keyboard_controller);
    WorkspacesModule::run(arc.clone(), vec![1, 2, 3, 4, 5, 6, 7]).await;
    // config_manager::write_config(config_path, &configuration);
}
