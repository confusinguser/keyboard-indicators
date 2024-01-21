use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

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
}

impl ModuleType {
    pub(crate) fn run(
        &self,
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
    ) -> Vec<JoinHandle<()>> {
        match self {
            ModuleType::WorkspacesModule => WorkspacesModule::run(keyboard_controller, module_leds),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "Sway Workspaces",
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "",
        }
    }

    pub(crate) fn all_module_types() -> [ModuleType; 1] {
        [ModuleType::WorkspacesModule]
    }
}
