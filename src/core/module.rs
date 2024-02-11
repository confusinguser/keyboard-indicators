use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::modules::media_playing::MediaModule;
use crate::modules::starfield::{StarfieldModule, StarfieldModuleOptions};
use crate::modules::workspaces::WorkspacesModule;

use super::keyboard_controller::KeyboardControllerMessage;

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
    Workspaces,
    Media,
    Starfield(StarfieldModuleOptions),
}

impl ModuleType {
    pub(crate) fn run(
        &self,
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        sender: Sender<KeyboardControllerMessage>,
        module_leds: Vec<Option<u32>>,
    ) {
        match self {
            ModuleType::Workspaces => {
                WorkspacesModule::run(task_tracker, cancellation_token, sender, module_leds)
            }
            ModuleType::Media => {
                MediaModule::run(task_tracker, cancellation_token, sender, module_leds)
            }
            ModuleType::Starfield(opts) => {
                StarfieldModule::run(task_tracker, cancellation_token, sender, module_leds, *opts)
            }
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "Sway Workspaces",
            ModuleType::Media => "Media Player Monitor",
            ModuleType::Starfield(_) => "Starfield Ambient Module",
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "",
            ModuleType::Media => "Shows media playhead and platform on keyboard",
            ModuleType::Starfield(_) => "",
        }
    }

    pub(crate) fn all_module_types() -> [ModuleType; 3] {
        [
            ModuleType::Workspaces,
            ModuleType::Media,
            ModuleType::Starfield(StarfieldModuleOptions::default()),
        ]
    }
}
