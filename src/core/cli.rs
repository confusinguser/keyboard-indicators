use std::path::PathBuf;

use clap::{arg, command, value_parser, ArgMatches, Args, Command, Parser, Subcommand};

// #[derive(Parser)] // requires `derive` feature
// #[command(name = "keyboard-indicators")]
// #[command(bin_name = "keyboard-indicators")]
// pub(crate) struct Cli {
//     #[command(subcommand)]
//     command: Commands,
// }

// #[derive(Subcommand, Clone)]
// pub(crate) enum Commands {
//     Start(StartArgs),
//     Config(ConfigArgs),
// }
// #[derive(Args, Clone)]
// #[command(version, about, long_about = None)]
// pub(crate) struct StartArgs {
//     /// Config path
//     #[arg(short, long)]
//     pub(crate) keymap_file: Option<PathBuf>,
// }

// #[derive(Args, Clone)]
// #[command(version, about, long_about = None)]
// pub(crate) struct ConfigArgs {
//     /// Config path
//     #[arg(short, long)]
//     pub(crate) keymap_file: Option<PathBuf>,

//     #[command(subcommand)]
//     command: ConfigCommands,
// }
// #[derive(Subcommand, Clone)]
// pub(crate) enum ConfigCommands {
//     AddModule(AddModuleArgs),
//     AddOverlayModule(AddOverlayModuleArgs),
//     RemoveModule(RemoveModuleArgs),
//     Create(CreateModuleArgs),
// }

pub(crate) fn parse_args() -> ArgMatches {
    command!() // requires `cargo` feature
        .subcommand_required(true)
        .arg(
            arg!(
                -c --config <FILE> "Sets a custom config file"
            )
            // We don't have syntax yet for optional options, so manually calling `required`
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .subcommands([
            Command::new("module").about("manage modules").subcommands([
                Command::new("add").about("Create new module"),
                Command::new("remove").about("Remove module"),
                Command::new("info").about("Get module info"),
            ]).subcommand_required(true),
            Command::new("start").about("Start keyboard indicator program"),
            Command::new("create-config").about("Make a new config file, overwriting any old ones").arg(
                arg!(
                    -l --ledlimit [n] "Limits configuration to the first n LED indicies. Useful when testing"
                )
                .required(false)
                .value_parser(value_parser!(u32))
            ).arg(
                arg!(
                    -c --config <FILE> "Sets a custom config file"
                )
                // We don't have syntax yet for optional options, so manually calling `required`
                .required(false)
                .value_parser(value_parser!(PathBuf)),
            )
            ,
        ])
        .get_matches()
}
