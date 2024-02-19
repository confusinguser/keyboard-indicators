use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::modules::media_playing::MediaModule;
use crate::modules::noise::{NoiseModule, NoiseModuleOptions};
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
    Noise(NoiseModuleOptions),
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
            ModuleType::Noise(opts) => {
                NoiseModule::run(task_tracker, cancellation_token, sender, module_leds, *opts)
            }
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "Sway Workspaces",
            ModuleType::Media => "Media Player Monitor",
            ModuleType::Starfield(_) => "Starfield Ambient",
            ModuleType::Noise(_) => "Noise",
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "",
            ModuleType::Media => "Shows media playhead and platform on keyboard",
            ModuleType::Starfield(_) => "",
            ModuleType::Noise(_) => "Noise thing",
        }
    }

    pub(crate) fn all_module_types() -> [ModuleType; 4] {
        [
            ModuleType::Workspaces,
            ModuleType::Media,
            ModuleType::Starfield(StarfieldModuleOptions::default()),
            ModuleType::Noise(NoiseModuleOptions::default()),
        ]
    }
}
