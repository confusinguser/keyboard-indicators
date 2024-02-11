use std::sync::Arc;

use clap::ArgMatches;
use tokio::signal;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::config_manager;
use crate::core::keyboard_controller::{KeyboardController, KeyboardControllerMessage};

pub(crate) async fn start(args: &ArgMatches) -> anyhow::Result<()> {
    let config = config_manager::read_config_and_keymap_from_args(args)?;
    let (sender, receiver) = mpsc::channel::<KeyboardControllerMessage>(100);
    let keyboard_controller = KeyboardController::connect().await?;

    let cancellation_token = CancellationToken::new();
    let keyboard_controller = Arc::new(Mutex::new(keyboard_controller));
    let task_tracker = TaskTracker::new();
    KeyboardController::run(
        keyboard_controller,
        &task_tracker,
        cancellation_token.clone(),
        receiver,
    );
    for module in &config.modules {
        module.module_type.run(
            &task_tracker,
            cancellation_token.clone(),
            sender.clone(),
            module.module_leds.clone(),
        );
    }

    task_tracker.close();

    match signal::ctrl_c().await {
        Ok(_) => {
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
