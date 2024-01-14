use std::sync::Arc;

use ksway::IpcEvent;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::core::constants;
use crate::core::keyboard_controller::KeyboardController;
use crate::core::module::LinearModule;
use crate::core::utils::run_command_async;

pub(crate) struct WorkspacesModule {
    sway_client: ksway::Client,
}

impl LinearModule for WorkspacesModule {
    fn run(
        keyboard_controller: Arc<KeyboardController>,
        leds_order: Vec<u32>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let stdout =
                run_command_async("swaymsg --pretty -t subscribe -m '[\"workspace\"]'").unwrap();
            let mut buffer = BufReader::new(stdout).lines();

            // A rudimentary form of parsing the JSON
            let mut is_old = false;
            let mut message_type = None;
            loop {
                let line = buffer
                    .next_line()
                    .await
                    .unwrap_or_default()
                    .unwrap_or_default();
                if line.is_empty() {
                    continue;
                }
                let line_trimmed = line.trim_start();

                dbg!(&line_trimmed);
                if line_trimmed.starts_with("\"change\":") {
                    let message_type_string = line.trim_start().split(':').last();
                    if let Some(message_type_string) = message_type_string {
                        if let Some(message_type_local) = MessageType::parse(message_type_string) {
                            message_type = Some(message_type_local);
                        }
                    }
                    continue;
                }

                if line_trimmed.starts_with("\"old\":") {
                    is_old = true;
                    continue;
                }

                if line_trimmed.starts_with("\"current\":") {
                    is_old = false;
                    continue;
                }

                if !line_trimmed.starts_with("\"num\":") {
                    continue;
                }

                let Some(Ok(num)) = line_trimmed.split(':').last().map(|num| num.parse::<u32>()) else {continue;};
                dbg!(num, is_old);

                if let Some(message_type) = message_type {
                    let Some(&led_index) = leds_order.get(num as usize - 1) else {continue;};
                    match message_type {
                        MessageType::Focus => {
                            let color = if is_old {
                                constants::UNFOCUSED_WORKSPACE_COLOR
                            } else {
                                constants::CURRENT_WORKSPACE_COLOR
                            };
                            keyboard_controller.set_led_index(led_index, color).await;
                        }
                        MessageType::Empty => {
                            keyboard_controller
                                .set_led_index(led_index, constants::EMPTY_WORKSPACE_COLOR)
                                .await;
                        }
                    }
                }
            }
        })
    }
}

impl WorkspacesModule {
    fn create_event_receiver(&self) -> Result {
        let sway_client = match self.sway_client {
            Some(c) => c,
            None => ksway::Client::connect()?,
        };
        let listener = sway_client.subscribe(vec![IpcEvent::Window]);
    }
}

#[derive(Clone, Copy)]
enum MessageType {
    Focus,
    Empty,
}

impl MessageType {
    fn parse(string: &str) -> Option<MessageType> {
        match string {
            "focus" => Some(Self::Focus),
            "empty" => Some(Self::Empty),
            _ => None,
        }
    }
}
