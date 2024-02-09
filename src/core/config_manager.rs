use std::path::PathBuf;
use std::{fs, io};

use anyhow::{bail, Context};
use clap::ArgMatches;
use serde::{Deserialize, Serialize};

use super::keymap::Keymap;
use super::module::Module;
use super::utils;

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Configuration {
    #[serde(skip_serializing)]
    #[serde(default)]
    pub(crate) keymap: Keymap,
    pub(crate) modules: Vec<Module>,
}

/// Returns None if the file is not found, an error in the case of another error, and the
/// deserialized config object in case the files are found
pub(crate) fn read_config_and_keymap(
    config_path: &PathBuf,
    keymap_path: &PathBuf,
) -> anyhow::Result<Configuration> {
    let contents = fs::read_to_string(config_path);
    let mut config;
    if let Err(error) = contents {
        if error.kind() == io::ErrorKind::NotFound {
            // eprintln!("Config not found. Creating it in {:?}", &config_path);
            config = Configuration::default();
        } else {
            bail!(error);
        }
    } else {
        let contents = contents?;
        config =
            serde_yaml::from_str::<Configuration>(&contents).context("Error reading the config")?;
    }

    let contents = fs::read_to_string(keymap_path)
        .context("There is no keymap file. Run the create-keymap subcommand to construct one.")?;
    let keymap = serde_yaml::from_str::<Keymap>(&contents)?;
    config.keymap = keymap;
    Ok(config)
}

pub(crate) fn read_config_and_keymap_from_args(args: &ArgMatches) -> anyhow::Result<Configuration> {
    let config_path = utils::get_config_path(args)?;
    let keymap_path = utils::get_keymap_path(args)?;
    read_config_and_keymap(&config_path, &keymap_path)
}

pub(crate) fn write_config(path: &PathBuf, configuration: &Configuration) -> anyhow::Result<()> {
    let contents = serde_yaml::to_string(configuration)?;
    fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn write_keymap(path: &PathBuf, keymap: &Keymap) -> anyhow::Result<()> {
    let contents = serde_yaml::to_string(keymap)?;
    fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn write_config_and_keymap(
    config_path: &PathBuf,
    keymap_path: &PathBuf,
    configuration: &Configuration,
) -> anyhow::Result<()> {
    write_config(config_path, configuration)?;
    write_keymap(keymap_path, &configuration.keymap)?;
    Ok(())
}

pub(crate) fn write_config_and_keymap_from_args(
    args: &ArgMatches,
    configuration: &Configuration,
) -> anyhow::Result<()> {
    let config_path = utils::get_config_path(args)?;
    let keymap_path = utils::get_keymap_path(args)?;
    write_config_and_keymap(&config_path, &keymap_path, configuration)?;
    Ok(())
}
