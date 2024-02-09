use std::sync::Arc;

use clap::ArgMatches;

use crate::core::config_manager;
use crate::core::keyboard_controller::KeyboardController;

pub(crate) async fn start(args: &ArgMatches) -> anyhow::Result<()> {
    let config = config_manager::read_config_and_keymap_from_args(args)?;
    let keyboard_controller = KeyboardController::connect(config).await?;

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

    // Make sure to not exit if threads are open
    for hook in join_hooks {
        hook.await?;
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
