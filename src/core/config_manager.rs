use core::fmt;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

pub(crate) struct ConfigReadError {
    serialization_error: bool,
    error_string: String,
}

impl fmt::Display for ConfigReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.serialization_error {
            true => write!(
                f,
                "Config error. Malformed configuration. Error message: {}",
                self.error_string
            ),
            false => write!(
                f,
                "Config error. File not found. Error message: {}",
                self.error_string
            ),
        }
    }
}

pub(crate) struct ConfigWriteError {
    error_string: String,
}

impl fmt::Display for ConfigWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Config write error. Error message: {}",
            self.error_string
        )
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub(crate) struct Configuration {
    pub(crate) key_led_map: HashMap<KeyCode, u32>,
    pub(crate) first_in_row: Vec<u32>,
    pub(crate) skip_indicies: BTreeSet<u32>,
}

pub(crate) fn read_config(path: &Path) -> Result<Configuration, ConfigReadError> {
    let contents = fs::read_to_string(path).map_err(|error| ConfigReadError {
        serialization_error: true,
        error_string: error.to_string(),
    })?;
    serde_yaml::from_str::<Configuration>(&contents).map_err(|error| ConfigReadError {
        serialization_error: true,
        error_string: error.to_string(),
    })
}

pub(crate) fn write_config(
    path: &Path,
    configuration: &Configuration,
) -> Result<(), ConfigWriteError> {
    let contents = serde_yaml::to_string(configuration).map_err(|error| ConfigWriteError {
        error_string: error.to_string(),
    })?;
    fs::write(path, contents).map_err(|error| ConfigWriteError {
        error_string: error.to_string(),
    })?;
    Ok(())
}
