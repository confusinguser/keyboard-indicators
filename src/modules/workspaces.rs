use std::borrow::Borrow;
use std::sync::Arc;

use anyhow::{bail, Context};
use openrgb::data::Color;
use rgb::{ComponentMap, RGB};
use swayipc_async::{Connection, EventType, WorkspaceChange, WorkspaceEvent};

use crate::core::keyboard_controller::KeyboardController;
use crate::core::module::LinearModule;
use crate::core::{constants, utils};
use futures_util::stream::StreamExt;

pub(crate) struct WorkspacesModule {}

impl LinearModule for WorkspacesModule {
    fn run(
        keyboard_controller: Arc<KeyboardController>,
        leds_order: Vec<u32>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let Ok(sway_client) = Connection::new().await else {
                eprintln!("Failed to connect to Sway socket");
                return;
            };
            let Ok(mut receiver) = sway_client.subscribe(vec![EventType::Workspace]).await else {
                eprintln!("Failed to subscribe to Sway events");
                return;
            };
            println!("Subscribed to Sway events");
            loop {
                let Some(Ok(message)) = receiver.next().await else {
                    continue;
                };
                match message {
                    swayipc_async::Event::Workspace(workspace_event) => {
                        let event_handler_output = Self::on_window_event(
                            &keyboard_controller,
                            &leds_order,
                            workspace_event,
                        )
                        .await;
                        if let Err(err) = event_handler_output {
                            eprintln!("{}", err)
                        };
                    }
                    swayipc_async::Event::Output(_) => todo!(),
                    swayipc_async::Event::Mode(_) => todo!(),
                    swayipc_async::Event::Window(_) => {}
                    swayipc_async::Event::BarConfigUpdate(_) => todo!(),
                    swayipc_async::Event::Binding(_) => todo!(),
                    swayipc_async::Event::Shutdown(_) => todo!(),
                    swayipc_async::Event::Tick(_) => todo!(),
                    swayipc_async::Event::BarStateUpdate(_) => todo!(),
                    swayipc_async::Event::Input(_) => todo!(),
                    _ => {}
                };
                // let Ok(event) = serde_json::from_slice(&message) else {
                //     eprintln!("Message received from Sway not valid serialised json");
                //     continue;
                // };
            }
        })
    }
}

impl WorkspacesModule {
    /// The event that is triggered whenever something happens with windows
    async fn on_window_event(
        keyboard_controller: &Arc<KeyboardController>,
        leds_order: &[u32],
        event: Box<WorkspaceEvent>,
    ) -> anyhow::Result<()> {
        Self::handle_workspace_change(keyboard_controller, leds_order, &event, false)
            .await
            .context("In current")?;
        Self::handle_workspace_change(keyboard_controller, leds_order, &event, true)
            .await
            .context("In old")?;

        Ok(())
    }

    async fn handle_workspace_change(
        keyboard_controller: &Arc<KeyboardController>,
        leds_order: &[u32],
        event: &WorkspaceEvent,
        old: bool,
    ) -> anyhow::Result<()> {
        let change: WorkspaceChange = event.change;
        let workspace = if old { &event.old } else { &event.current };
        let Some(workspace) = workspace else { return Ok(()); };
        let Some(workspace_num)= workspace.num else { bail!("Workspace exists but has no number") };
        let all_app_colors = workspace
            .nodes
            .iter()
            .filter_map(|e| {
                if e.app_id.as_ref().is_some_and(|app_id| app_id.is_empty()) {
                    return Some(constants::SPOTIFY_WORKSPACE_COLOR);
                }
                if e.app_id.is_none() {
                    return Some(constants::DISCORD_WORKSPACE_COLOR);
                }
                if e.app_id.as_ref().is_some_and(|app_id| app_id == "firefox") {
                    return Some(constants::FIREFOX_WORKSPACE_COLOR);
                }
                None
            })
            .collect::<Vec<Color>>();

        let mut average_color_u32 = RGB::new(0_u32, 0, 0);
        for app_color in all_app_colors.iter() {
            average_color_u32 += app_color.map(|comp| comp as u32);
        }

        #[allow(clippy::len_zero)]
        let average_color = if all_app_colors.len() != 0 {
            average_color_u32.map(|comp| (comp / all_app_colors.len() as u32) as u8)
        } else {
            constants::UNFOCUSED_DEFAULT_WORKSPACE_COLOR
        };

        let Some(&led_index) = leds_order.get(workspace_num as usize - 1) else { bail!("Workspace outside the LED range") };
        let new_color = match change {
            WorkspaceChange::Focus => {
                if old {
                    Some(utils::overlay(
                        constants::UNFOCUSED_WORKSPACE_OVERLAY,
                        average_color,
                    ))
                } else {
                    Some(utils::overlay(
                        constants::CURRENT_WORKSPACE_OVERLAY,
                        average_color,
                    ))
                }
            }
            WorkspaceChange::Empty => Some(constants::EMPTY_WORKSPACE_COLOR),
            WorkspaceChange::Init => Some(constants::UNFOCUSED_DEFAULT_WORKSPACE_COLOR),
            WorkspaceChange::Move => todo!(),
            WorkspaceChange::Urgent => Some(constants::URGENT_WORKSPACE_COLOR),
            WorkspaceChange::Reload => todo!(),
            _ => None,
        };
        if let Some(new_color) = new_color {
            keyboard_controller
                .set_led_index(led_index, new_color)
                .await?;
        }
        Ok(())
    }
}
