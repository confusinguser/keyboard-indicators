use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use openrgb::data::{Color, Controller};
use openrgb::{OpenRGB, OpenRGBError};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub(crate) struct KeyboardController {
    client: OpenRGB<TcpStream>,
    controller_id: u32,
    controller: Controller,
    current_colors: Vec<Color>,
}

impl KeyboardController {
    pub(crate) async fn update_led_urgent(
        &mut self,
        sender: &mut Sender<(u32, Color)>,
        index: u32,
        color: Color,
    ) -> Result<(), OpenRGBError> {
        sender.send((index, color));
        self.client
            .update_led(self.controller_id, index as i32, color)
            .await
    }

    pub(crate) async fn update_led(sender: &mut Sender<(u32, Color)>, index: u32, color: Color) {
        sender.send((index, color));
    }

    pub(crate) async fn connect() -> anyhow::Result<Self> {
        let client = OpenRGB::connect().await;
        if client.is_err() {
            panic!("Connection refused. Check that OpenRGB is running")
        }
        let client = client.unwrap();
        let controller_id = 1;
        let controller = client.get_controller(controller_id).await?;
        Ok(KeyboardController {
            client,
            controller_id,
            controller,
            current_colors: Vec::new(),
        })
    }

    pub(crate) async fn turn_all_off(&mut self) -> anyhow::Result<()> {
        let num_leds = self.num_leds();
        for i in 0..num_leds as usize {
            self.current_colors.insert(i, Color::new(0, 0, 0));
        }
        self.client
            .update_leds(
                self.controller_id,
                vec![Color::new(0, 0, 0); num_leds as usize],
            )
            .await?;
        Ok(())
    }

    pub(crate) fn num_leds(&self) -> u32 {
        self.controller.leds.len() as u32
    }

    pub(crate) fn run(
        keyboard_controller: Arc<Mutex<KeyboardController>>,
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        receiver: Receiver<(u32, Color)>,
    ) {
        task_tracker.spawn(async move {
            let mut all_messages: Vec<(u32, Color)> = Vec::with_capacity(100);
            loop {
                // Get all messages that are currently in the receiver (max 100)
                loop {
                    let message = tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(10)) => None,
                        val = async {
                            if all_messages.len() < 100 {
                                receiver.recv().await
                            } else {
                                println!("Over 100 messages were taken in the same cycle");
                                None
                            }
                        } => val
                    };
                    if let Some(message) = message {
                        all_messages.push(message);
                    } else {
                        break;
                    }
                }

                let lock = keyboard_controller
                    .lock()
                    .expect("Mutex is not acquireable");
                let current_colors = lock.current_colors;
                lock.client
                    .update_leds(lock.controller_id, current_colors)
                    .await;
            }
        });
    }
}
