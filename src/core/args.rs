use clap::{arg, command, Parser};

#[derive(Parser)] // requires `derive` feature
#[command(name = "keyboard-indicators")]
#[command(bin_name = "keyboard-indicators")]
enum Cli {
    Start(StartArgs),
}
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct StartArgs {
    /// Config path
    #[arg(
        short,
        long,
        default_value = "~/.config/keyboard-indicators/keymap.yaml"
    )]
    pub(crate) keymap_path: Option<String>,
}

pub(crate) fn parse_args() -> StartArgs {
    let Cli::Start(args) = Cli::parse();
    args
}
