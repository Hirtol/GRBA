use crate::rendering::gui::debug::DebugView;
use crate::rendering::gui::debug::memory_view::DebugMemoryEditor;

/// Represents a special (and possibly expensive) request for debug information to
/// the emulator thread.
#[derive(Debug)]
pub enum DebugMessageUi {
    MemoryRequest(<DebugMemoryEditor as DebugView>::RequestInformation, Option<<DebugMemoryEditor as DebugView>::EmuUpdate>),
}

/// Represents the response to a [DebugMessageUi] request.
#[derive(Debug)]
pub enum DebugMessageResponse {
    MemoryResponse(<DebugMemoryEditor as DebugView>::RequestedData),
}