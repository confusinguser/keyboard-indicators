use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use openrgb::data::{Color, Controller};
use openrgb::{OpenRGB, OpenRGBError};
use rgb::RGB;
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
    ) -> anyhow::Result<()> {
        sender.send((index, color)).await?;
        self.client
            .update_led(self.controller_id, index as i32, color)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_led(
        sender: &mut Sender<(u32, Color)>,
        index: u32,
        color: Color,
    ) -> anyhow::Result<()> {
        sender.send((index, color)).await?;
        Ok(())
    }

    pub(crate) async fn connect() -> anyhow::Result<Self> {
        let client = OpenRGB::connect().await;
        if client.is_err() {
            panic!("Connection refused. Check that OpenRGB is running")
        }
        let client = client.unwrap();
        let controller_id = 1;
        let controller = client.get_controller(controller_id).await?;

        let current_colors = vec![Color::new(0, 0, 0); controller.leds.len()];
        Ok(KeyboardController {
            client,
            controller_id,
            controller,
            current_colors,
        })
    }

    /// Assumes that self.current_colors is num_leds long
    pub(crate) async fn turn_all_off(&mut self) -> anyhow::Result<()> {
        let num_leds = self.num_leds();
        for i in 0..num_leds as usize {
            self.current_colors[i] = Color::new(0, 0, 0);
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
        mut receiver: Receiver<(u32, Color)>,
    ) {
        task_tracker.spawn(async move {
            let mut lock = keyboard_controller.lock().await;
            lock.turn_all_off().await.unwrap();
            let mut all_messages: Vec<(u32, Color)> = Vec::with_capacity(100);
            for _ in lock.current_colors.len()..lock.num_leds() as usize {
                lock.current_colors.push(RGB::from((0, 0, 0)));
            }
            drop(lock);

            loop {
                if cancellation_token.is_cancelled() {
                    break;
                }
                // Get all messages that are currently in the receiver (max 100)
                loop {
                    let all_messages_len = all_messages.len();
                    let message = tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(10)) => None,
                        val = async {
                            if all_messages_len < 100 {
                                receiver.recv().await
                            } else {
                                println!("Over 100 messages were received in the same cycle");
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
                if all_messages.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                // Updating LED colors
                let mut lock = keyboard_controller.lock().await;
                for message in &all_messages {
                    if message.0 as usize >= lock.current_colors.len() {
                        continue;
                    }
                    lock.current_colors[message.0 as usize] = message.1;
                }
                assert_eq!(lock.current_colors.len(), lock.num_leds() as usize);
                lock.client
                    .update_leds(lock.controller_id, lock.current_colors.clone())
                    .await
                    .unwrap();
                drop(lock);

                all_messages.clear();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
    }
}
