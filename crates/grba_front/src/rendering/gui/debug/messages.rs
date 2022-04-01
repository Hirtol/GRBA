use crate::rendering::gui::debug::cpu_state::CpuStateView;
use crate::rendering::gui::debug::memory_view::MemoryEditorView;
use crate::rendering::gui::debug::DebugView;

/// Represents a special (and possibly expensive) request for debug information to
/// the emulator thread.
#[derive(Debug)]
pub enum DebugMessageUi {
    MemoryRequest(
        <MemoryEditorView as DebugView>::RequestInformation,
        Option<<MemoryEditorView as DebugView>::EmuUpdate>,
    ),
    CpuRequest(<CpuStateView as DebugView>::RequestInformation),
}

/// Represents the response to a [DebugMessageUi] request.
#[derive(Debug)]
pub enum DebugMessageResponse {
    MemoryResponse(<MemoryEditorView as DebugView>::RequestedData),
    CpuResponse(<CpuStateView as DebugView>::RequestedData),
}
