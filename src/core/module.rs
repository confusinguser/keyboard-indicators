use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::modules::media_playing::MediaModule;
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
}

impl ModuleType {
    pub(crate) fn run(
        &self,
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        keyboard_controller: Arc<KeyboardController>,
        module_leds: Vec<Option<u32>>,
    ) {
        match self {
            ModuleType::WorkspacesModule => WorkspacesModule::run(
                task_tracker,
                cancellation_token,
                keyboard_controller,
                module_leds,
            ),
            ModuleType::MediaModule => MediaModule::run(
                task_tracker,
                cancellation_token,
                keyboard_controller,
                module_leds,
            ),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "Sway Workspaces",
            ModuleType::MediaModule => "Media Player Monitor",
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::WorkspacesModule => "",
            ModuleType::MediaModule => "Shows media playhead and platform on keyboard",
        }
    }

    pub(crate) fn all_module_types() -> [ModuleType; 2] {
        [ModuleType::WorkspacesModule, ModuleType::MediaModule]
    }
}
