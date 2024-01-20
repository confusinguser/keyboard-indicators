use std::sync::Arc;

use tokio::task::JoinHandle;

use super::keyboard_controller::KeyboardController;

pub(crate) trait LinearModule {
    fn run(keyboard_controller: Arc<KeyboardController>, rgbs_in_order: Vec<u32>)
        -> JoinHandle<()>;
}
