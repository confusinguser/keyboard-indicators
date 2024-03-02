use std::sync::Arc;
use std::time::Duration;

use openrgb::data::{Color, Controller};
use openrgb::OpenRGB;
use rgb::RGB;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::core::utils::compute_light_curve_for_color;

#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyboardControllerMessage {
    led: Option<u32>,
    color: Color,
    urgent: bool,
}

impl KeyboardControllerMessage {
    fn new(led: u32, color: Color, urgent: bool) -> Self {
        Self {
            led: Some(led),
            color,
            urgent,
        }
    }
    fn new_global(color: Color, urgent: bool) -> Self {
        Self {
            led: None,
            color,
            urgent,
        }
    }
}

pub(crate) struct KeyboardController {
    client: OpenRGB<TcpStream>,
    controller_id: u32,
    controller: Controller,
    current_colors: Vec<Color>,
}

impl KeyboardController {
    pub(crate) async fn update_led_urgent(
        sender: &mut Sender<KeyboardControllerMessage>,
        index: u32,
        color: Color,
    ) -> anyhow::Result<()> {
        sender
            .send(KeyboardControllerMessage::new(index, color, true))
            .await?;
        Ok(())
    }

    pub(crate) async fn update_led(
        sender: &mut Sender<KeyboardControllerMessage>,
        index: u32,
        color: Color,
    ) -> anyhow::Result<()> {
        sender
            .send(KeyboardControllerMessage::new(index, color, false))
            .await?;
        Ok(())
    }

    pub(crate) async fn update_all_leds(
        sender: &mut Sender<KeyboardControllerMessage>,
        color: Color,
        urgent: bool,
    ) -> anyhow::Result<()> {
        sender
            .send(KeyboardControllerMessage::new_global(color, urgent))
            .await?;
        Ok(())
    }
    pub(crate) async fn turn_all_off(
        sender: &mut Sender<KeyboardControllerMessage>,
    ) -> anyhow::Result<()> {
        Self::update_all_leds(sender, Color::new(0, 0, 0), false).await?;
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

    pub(crate) fn num_leds(&self) -> u32 {
        self.controller.leds.len() as u32
    }

    pub(crate) fn run(
        keyboard_controller: Arc<Mutex<KeyboardController>>,
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        mut receiver: Receiver<KeyboardControllerMessage>,
    ) {
        task_tracker.spawn(async move {
            let mut lock = keyboard_controller.lock().await;
            let mut all_messages: Vec<KeyboardControllerMessage> = Vec::with_capacity(200);
            for _ in lock.current_colors.len()..lock.num_leds() as usize {
                lock.current_colors.push(RGB::from((0, 0, 0)));
            }
            drop(lock);
            let mut time_of_last_update = Instant::now();

            loop {
                if cancellation_token.is_cancelled() {
                    break;
                }
                // Get all messages that are currently in the receiver (max 200)
                let mut time_of_urgent_flag = None;
                loop {
                    let all_messages_len = all_messages.len();
                    if all_messages_len >= 200 {
                        println!("Over 200 messages were received in the same cycle");
                        break;
                    }
                    let sleep_until = if let Some(urgent_flag) = time_of_urgent_flag {
                        urgent_flag + Duration::from_millis(1)
                    } else {
                        time_of_last_update + Duration::from_millis(10)
                    };
                    let message = tokio::select! {
                        _ = tokio::time::sleep_until(sleep_until) => None,
                        val = receiver.recv() => val
                    };
                    if let Some(message) = message {
                        if message.led.is_none() {
                            // The current message changes all leds
                            all_messages.clear();
                        }
                        all_messages.push(message);
                        if message.urgent && time_of_urgent_flag.is_none() {
                            time_of_urgent_flag = Some(Instant::now());
                        }
                    } else {
                        break;
                    }
                }
                if all_messages.is_empty() {
                    continue;
                }

                // Updating LED colors
                let mut lock = keyboard_controller.lock().await;
                for message in &all_messages {
                    if let Some(led) = message.led {
                        if led as usize >= lock.current_colors.len() {
                            continue;
                        }
                        lock.current_colors[led as usize] =
                            compute_light_curve_for_color(30000., message.color);
                    } else {
                        for i in 0..lock.current_colors.len() {
                            lock.current_colors[i] = Color::new(0, 0, 0);
                        }
                    }
                }
                if !all_messages.is_empty() {
                    lock.client
                        .update_leds(lock.controller_id, lock.current_colors.clone())
                        .await
                        .unwrap();
                }
                drop(lock);

                time_of_last_update = Instant::now();
                all_messages.clear();
            }
        });
    }
}
