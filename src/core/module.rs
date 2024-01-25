use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::modules::media_playing::MediaModule;
use crate::modules::starfield::{StarfieldModule, StarfieldModuleOptions};
use crate::modules::workspaces::WorkspacesModule;

use super::keyboard_controller::KeyboardController;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Module {
    pub(crate) module_type: ModuleType,
    pub(crate) module_leds: Vec<Option<u32>>,
}
impl Module {
    pub(crate) fn new(module_type: ModuleType, module_leds: Vec<Option<u32>>) -> Self {
        Self {
            module_type,
            module_leds,
        }
    }
}

#[derive(Serialize, Debug, Copy, Clone, Deserialize)]
pub(crate) enum ModuleType {
    WorkspacesModule,
    MediaModule,
    StarfieldModule(StarfieldModuleOptions),
}

impl ModuleType {
    pub(crate) fn run(
        &self,
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
    ) -> Vec<JoinHandle<()>> {
        match self {
            ModuleType::WorkspacesModule => WorkspacesModule::run(keyboard_controller, module_leds),
            ModuleType::MediaModule => MediaModule::run(keyboard_controller, module_leds),
            ModuleType::StarfieldModule(opts) => {
                StarfieldModule::run(keyboard_controller, module_leds, *opts)
            }
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "Sway Workspaces",
            ModuleType::MediaModule => "Media Player Monitor",
            ModuleType::StarfieldModule(_) => "Starfield Ambient Module",
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "",
            ModuleType::MediaModule => "Shows media playhead and platform on keyboard",
            ModuleType::StarfieldModule(_) => "",
        }
    }

    pub(crate) fn all_module_types() -> [ModuleType; 2] {
        [ModuleType::WorkspacesModule, ModuleType::MediaModule]
    }
}
