use crate::rendering::gui::DebugMessage;
use grba_core::InputKeys;

#[derive(Debug)]
pub enum EmulatorMessage {
    /// Informs the emulator thread that it should exit.
    ExitRequest,
    Debug(DebugMessage),
    KeyDown(InputKeys),
    KeyUp(InputKeys),
}

#[derive(Debug)]
pub enum EmulatorResponse {
    Debug(DebugMessage),
}
