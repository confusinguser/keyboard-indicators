use clap::{arg, command, Parser};

#[derive(Parser)] // requires `derive` feature
#[command(name = "keyboard-indicators")]
#[command(bin_name = "keyboard-indicators")]
pub(crate) enum Cli {
    Start(StartArgs),
    CreateConfig(CreateConfigArgs),
}
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct StartArgs {
    /// Config path
    #[arg(short, long)]
    pub(crate) keymap_path: Option<String>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct CreateConfigArgs {
    /// Config path
    #[arg(short, long)]
    pub(crate) keymap_path: Option<String>,
}

pub(crate) fn parse_args() -> Cli {
    Cli::parse()
}
