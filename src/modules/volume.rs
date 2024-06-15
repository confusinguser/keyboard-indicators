use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

use rgb::RGB8;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, State};
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::operation;
use libpulse_binding::operation::Operation;

use crate::core::keyboard_controller::KeyboardControllerMessage;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct VolumeModuleOptions {
    color: RGB8,
    color_muted: RGB8,
    background_color: RGB8,
    max_percentage: u32,
}

impl Default for VolumeModuleOptions {
    fn default() -> Self {
        Self {
            color: RGB8::new(255, 255, 255),
            color_muted: RGB8::new(255, 0, 0),
            background_color: RGB8::new(0, 0, 0),
            max_percentage: 100,
        }
    }
}

pub(crate) struct VolumeModule {}

impl VolumeModule {
    pub(crate) fn run(
        task_tracker: &TaskTracker,
        cancellation_token: CancellationToken,
        sender: Sender<KeyboardControllerMessage>,
        module_leds: Vec<Option<u32>>,
        options: VolumeModuleOptions,
    ) {
        task_tracker.spawn(async move {
            let mut mainloop = Mainloop::new().unwrap();
            let context = Context::new(&mainloop, "KeyboardController").unwrap();

            loop {
                match mainloop.iterate(false) {
                    IterateResult::Success(_) => {}
                    IterateResult::Quit(retval) => anyhow::bail!("Mainloop quit. {:?}", retval),
                    IterateResult::Err(err) => anyhow::bail!("Mainloop error. {:?}", err),
                }

                if cancellation_token.is_cancelled() {
                    break;
                }

                match context.get_state() {
                    State::Ready => break,
                    State::Failed => anyhow::bail!("Context failed."),
                    State::Terminated => anyhow::bail!("Context terminated."),
                    _ => {}
                }
            }

            let introspect = context.introspect();
            let output: RefCell<Vec<SinkInfo>> = RefCell::new(Vec::new());
            let op = introspect.get_sink_info_by_index(
                0,
                |o| {
                    if let ListResult::Item(o) = o {
                        <Vec<SinkInfo> as AsMut<Vec<SinkInfo>>>::as_mut(&mut output.borrow_mut()).push(*o);
                    }
                },
            );
            Self::wait_for_operation(&mut mainloop, op).unwrap();
            println!("Current volume: {:?}", output.borrow());
            Ok(())
        });
    }

    pub fn wait_for_operation<G: ?Sized>(
        mainloop: &mut Mainloop,
        op: Operation<G>,
    ) -> anyhow::Result<()> {
        loop {
            match mainloop.iterate(false) {
                IterateResult::Err(e) => return Err(e.into()),
                IterateResult::Success(_) => {}
                IterateResult::Quit(_) => {
                    anyhow::bail!("Iterate state quit without an error");
                }
            }
            match op.get_state() {
                operation::State::Done => {
                    break;
                }
                operation::State::Running => {}
                operation::State::Cancelled => {
                    anyhow::bail!("Operation cancelled without an error");
                }
            }
        }
        Ok(())
    }

    fn state_callback(context: Rc<Mutex<Context>>) {
        println!("Try to lock context.");
        // let mut context = context.lock().unwrap();
        // let state = context.get_state();
        // println!("Callback called. {:?}", state);
        // match state {
        //     State::Ready => {
        //         println!("Context is ready.");
        //         Self::subscribe_events(
        //             context.deref_mut(),
        //             Box::new(|| {
        //                 println!("Context updated.");
        //             }),
        //         );
        //     }
        //     State::Failed | State::Terminated => {}
        //     _ => {}
        // }
    }
}
