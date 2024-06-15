use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::utils;
use crate::core::utils::rgb_to_hex;
use crate::modules::media_playing::MediaModule;
use crate::modules::noise::{NoiseModule, NoiseModuleOptions};
use crate::modules::starfield::{StarfieldModule, StarfieldModuleOptions};
use crate::modules::volume::{VolumeModule, VolumeModuleOptions};
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
    Volume(VolumeModuleOptions),
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
            ModuleType::Volume(opts) => {
                VolumeModule::run(task_tracker, cancellation_token, sender, module_leds, *opts)
            }
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "Sway Workspaces",
            ModuleType::Media => "Media Player Monitor",
            ModuleType::Starfield(_) => "Starfield Ambient",
            ModuleType::Noise(_) => "Noise",
            ModuleType::Volume(_) => "Volume Module"
        }
    }
    pub(crate) fn desc(&self) -> &'static str {
        match self {
            ModuleType::Workspaces => "",
            ModuleType::Media => "Shows media playhead and platform on keyboard",
            ModuleType::Starfield(_) => "",
            ModuleType::Noise(_) => "Noise thing",
            ModuleType::Volume(_) => "Volume module"
        }
    }
    pub(crate) fn add_all_settings(&self) -> (Vec<String>, Vec<Box<fn(&mut ModuleType)>>) {
        let mut choices_names: Vec<String> = Vec::new();
        let mut choices_handlers: Vec<Box<fn(&mut ModuleType)>> = Vec::new();
        macro_rules! add_choice {
            ($val: expr, $name: expr, $handler: expr) => {
                choices_names.push(format!("{} [Current: {:?}]", $name, $val));
                choices_handlers.push(Box::new($handler));
            };
        }
        match self {
            ModuleType::Workspaces => todo!(),
            ModuleType::Media => todo!(),
            ModuleType::Starfield(opts) => {
                add_choice!(rgb_to_hex(opts.background), "Background", |opts| {
                    if let ModuleType::Starfield(ref mut opts) = opts {
                        opts.background = utils::get_color_input().unwrap()
                    }
                });
                add_choice!(rgb_to_hex(opts.target_color), "Target color", |opts| {
                    if let ModuleType::Starfield(ref mut opts) = opts {
                        opts.target_color = utils::get_color_input().unwrap()
                    }
                });
                add_choice!(opts.animation_time, "Animation time (in seconds)", |opts| {
                    if let ModuleType::Starfield(ref mut opts) = opts {
                        opts.animation_time = Duration::from_secs_f64(
                            utils::get_input("Invalid number", |input| input.parse::<f64>().ok())
                                .unwrap(),
                        )
                    }
                });
                add_choice!(opts.time_variation, "Time variation", |opts| {
                    if let ModuleType::Starfield(ref mut opts) = opts {
                        opts.time_variation =
                            utils::get_input("Invalid number", |input| input.parse::<f32>().ok())
                                .unwrap();
                    }
                });
            }
            ModuleType::Noise(opts) => {
                add_choice!(rgb_to_hex(opts.color1), "First color", |opts| {
                    if let ModuleType::Noise(ref mut opts) = opts {
                        opts.color1 = utils::get_color_input().unwrap()
                    }
                });
                add_choice!(rgb_to_hex(opts.color2), "Second color", |opts| {
                    if let ModuleType::Noise(ref mut opts) = opts {
                        opts.color2 = utils::get_color_input().unwrap()
                    }
                });
                add_choice!(opts.speed, "Speed", |opts| {
                    if let ModuleType::Noise(ref mut opts) = opts {
                        opts.speed =
                            utils::get_input("Invalid number", |input| input.parse::<f32>().ok())
                                .unwrap();
                    }
                });
                add_choice!(opts.zoom_factor, "Zoom factor", |opts| {
                    if let ModuleType::Noise(ref mut opts) = opts {
                        opts.zoom_factor =
                            utils::get_input("Invalid number", |input| input.parse::<f32>().ok())
                                .unwrap();
                    }
                });
            }
            ModuleType::Volume(_) => { /*TODO*/ }
        };
        (choices_names, choices_handlers)
    }
    pub(crate) fn all_module_types() -> [ModuleType; 5] {
        [
            ModuleType::Workspaces,
            ModuleType::Media,
            ModuleType::Starfield(StarfieldModuleOptions::default()),
            ModuleType::Noise(NoiseModuleOptions::default()),
            ModuleType::Volume(VolumeModuleOptions::default()),
        ]
    }
}
