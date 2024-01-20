use std::sync::Arc;

use anyhow::{bail, Context};
use swayipc_async::{Connection, EventType, Node, WorkspaceChange, WorkspaceEvent};

use crate::core::constants;
use crate::core::keyboard_controller::KeyboardController;
use crate::core::module::LinearModule;
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
                println!("Received Sway message: {:?}", &message);
                match message {
                    swayipc_async::Event::Workspace(workspace_event) => {
                        dbg!(&workspace_event);
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
        Self::handle_workspace_change(
            keyboard_controller,
            leds_order,
            &event.current,
            event.change,
            false,
        )
        .await
        .context("In current")?;
        Self::handle_workspace_change(
            keyboard_controller,
            leds_order,
            &event.old,
            event.change,
            true,
        )
        .await
        .context("In old")?;

        Ok(())
    }

    async fn handle_workspace_change(
        keyboard_controller: &Arc<KeyboardController>,
        leds_order: &[u32],
        workspace: &Option<Node>,
        change: WorkspaceChange,
        old: bool,
    ) -> anyhow::Result<()> {
        let Some(workspace )= workspace else { return Ok(()); };
        let Some(workspace_name)= workspace.name.clone() else { bail!("Workspace exists but has no name") };
        let Some(Ok(num)) = workspace_name.split(':').last().map(|num| num.parse::<u32>()) else {bail!("workspace was a String but not a number")};

        let Some(&led_index) = leds_order.get(num as usize - 1) else {bail!("Workspace outside the LED range")};
        keyboard_controller
            .set_led_index(
                led_index,
                match change {
                    WorkspaceChange::Focus => {
                        if old {
                            constants::UNFOCUSED_WORKSPACE_COLOR
                        } else {
                            constants::CURRENT_WORKSPACE_COLOR
                        }
                    }
                    WorkspaceChange::Empty => constants::EMPTY_WORKSPACE_COLOR,
                    WorkspaceChange::Init => constants::UNFOCUSED_WORKSPACE_COLOR,
                    WorkspaceChange::Move => todo!(),
                    WorkspaceChange::Rename => todo!(),
                    WorkspaceChange::Urgent => todo!(),
                    WorkspaceChange::Reload => todo!(),
                    _ => todo!(),
                },
            )
            .await?;
        Ok(())
    }
}
