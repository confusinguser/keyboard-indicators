use std::path::PathBuf;

use clap::{arg, command, value_parser, ArgMatches, Command};

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
        ])
        .get_matches()
}
