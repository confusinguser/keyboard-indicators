use std::collections::{BTreeSet, HashMap};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Keymap {
    pub(crate) key_led_map: HashMap<KeyCode, u32>,
    pub(crate) first_in_row: Vec<u32>,
    pub(crate) skip_indicies: BTreeSet<u32>,
}
