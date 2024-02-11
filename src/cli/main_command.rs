use std::path::PathBuf;
use std::sync::Arc;

use anyhow::bail;
use clap::{arg, command, value_parser, ArgMatches, Command};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::keyboard_controller::KeyboardController;
use crate::core::{config_creator, config_manager, utils};

use super::module_subcommand;
use super::start_subcommand;

pub(crate) fn parse_args() -> ArgMatches {
    command!() // requires `cargo` feature
        .subcommand_required(true)
        .arg(
            arg!(
                -c --config <FILE> "Sets a custom config file"
            )
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .subcommands([
            Command::new("module").about("manage modules").subcommands([
                Command::new("add").about("Create new module"),
                Command::new("remove").about("Remove module"),
                Command::new("info").about("Get module info"),
                Command::new("modify").about("Modify a module"),
            ]).subcommand_required(true),
            Command::new("start").about("Start keyboard indicator program"),
            Command::new("create-config").about("Make a new config file, overwriting any old ones").arg(
                arg!(
                    -l --ledlimit [n] "Limits configuration to the first n LED indicies. Useful when testing"
                )
                .required(false)
                .value_parser(value_parser!(u32))
            )
            ,
            Command::new("create-keymap").about("Creates a map between each key and its corresponding LED.").arg(
                arg!(
                    -l --ledlimit [n] "Limits configuration to the first n LED indicies. Useful when testing"
                )
                .required(false)
                .value_parser(value_parser!(u32))
            )
        ])
        .get_matches()
}

pub(crate) async fn main_command(matches: ArgMatches) -> anyhow::Result<()> {
    // TODO: Avoid unwrap
    match matches.subcommand_name() {
        Some("start") => start_subcommand::start(matches.subcommand().unwrap().1).await,
        Some("create-config") => create_config(matches.subcommand().unwrap().1).await,
        Some("module") => module_subcommand::module(matches.subcommand().unwrap().1).await,
        Some("create-keymap") => create_keymap(matches.subcommand().unwrap().1).await,
        _ => bail!("Unknown subcommand"),
    }
}

async fn create_config(args: &ArgMatches) -> anyhow::Result<()> {
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
    let new_config = config_creator::start_config_creator(
        keyboard_controller,
        &mut sender,
        args.get_one::<u32>("ledlimit").copied(),
    )
    .await?;
    config_manager::write_config_and_keymap_from_args(args, &new_config)?;
    Ok(())
}

async fn create_keymap(args: &ArgMatches) -> anyhow::Result<()> {
    // TODO Confirm that the user wants to do this
    let keymap_path = utils::get_keymap_path(args)?;
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
    let new_keymap = config_creator::create_keymap(
        keyboard_controller,
        &mut sender,
        args.get_one::<u32>("ledlimit").copied(),
    )
    .await?;

    config_manager::write_keymap(&keymap_path, &new_keymap)?;
    println!("The keymap has been saved. You can now use the module command to configure modules");
    Ok(())
}
