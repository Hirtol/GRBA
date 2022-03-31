use crate::rendering::gui::{DebugMessageResponse, DebugMessageUi};
use grba_core::InputKeys;

#[derive(Debug)]
pub enum EmulatorMessage {
    /// Informs the emulator thread that it should exit.
    ExitRequest,
    Debug(DebugMessageUi),
    KeyDown(InputKeys),
    KeyUp(InputKeys),
    Pause,
    Unpause,
}

#[derive(Debug)]
pub enum EmulatorResponse {
    Debug(DebugMessageResponse),
}
