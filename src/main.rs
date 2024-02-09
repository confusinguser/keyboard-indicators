use self::cli::main_command;

mod cli;
mod core;
mod modules;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    main_command::main_command(main_command::parse_args()).await?;
    Ok(())
}
