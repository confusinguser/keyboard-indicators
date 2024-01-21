use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Configuration {
    pub(crate) key_led_map: HashMap<KeyCode, u32>,
    pub(crate) first_in_row: Vec<u32>,
    pub(crate) skip_indicies: BTreeSet<u32>,
}

pub(crate) fn read_config(path: &PathBuf) -> anyhow::Result<Configuration> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str::<Configuration>(&contents)?)
}

pub(crate) fn write_config(path: &PathBuf, configuration: &Configuration) -> anyhow::Result<()> {
    let contents = serde_yaml::to_string(configuration)?;
    fs::write(path, contents)?;
    Ok(())
}
