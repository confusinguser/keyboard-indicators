use openrgb::data::{Color, Controller};
use openrgb::{OpenRGB, OpenRGBError};
use tokio::net::TcpStream;

use super::config_manager::Configuration;

pub(crate) struct KeyboardController {
    client: OpenRGB<TcpStream>,
    controller_id: u32,
    controller: Controller,
    pub(crate) config: Configuration,
}

impl KeyboardController {
    pub(crate) async fn set_led_index(&self, index: u32, color: Color) -> Result<(), OpenRGBError> {
        // let first_in_row = vec![0, 23, 44, 67, 70, 90];
        // let id = first_in_row[y as usize] + x;

        self.client
            .update_led(self.controller_id, index as i32, color)
            .await
    }

    pub(crate) async fn set_led(&self, x: u32, y: u32, color: Color) -> Result<(), OpenRGBError> {
        let mut index = self.config.first_in_row[y as usize] + x;
        for &skip_index in self.config.skip_indicies.iter() {
            if skip_index <= index {
                index += 1;
            }
        }

        self.client
            .update_led(self.controller_id, index as i32, color)
            .await
    }

    pub(crate) async fn connect(configuration: Configuration) -> anyhow::Result<Self> {
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
            config: configuration,
        })
    }

    pub(crate) async fn turn_all_off(&self) -> anyhow::Result<()> {
        self.client
            .update_leds(
                self.controller_id,
                vec![Color::new(0, 0, 0); self.num_leds().await as usize],
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn num_leds(&self) -> u32 {
        self.controller.leds.len() as u32
    }
}
