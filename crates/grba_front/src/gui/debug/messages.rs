use crate::gui::debug::cpu_state_view::CpuStateView;
use crate::gui::debug::memory_view::MemoryEditorView;
use crate::gui::debug::DebugView;
use crate::gui::debug::palette_view::PaletteView;

/// Represents a special (and possibly expensive) request for debug information to
/// the emulator thread.
#[derive(Debug)]
pub enum DebugMessageUi {
    MemoryRequest(
        <MemoryEditorView as DebugView>::RequestInformation,
        Option<<MemoryEditorView as DebugView>::EmuUpdate>,
    ),
    CpuRequest(<CpuStateView as DebugView>::RequestInformation),
    PaletteRequest(<PaletteView as DebugView>::RequestInformation),
}

/// Represents the response to a [DebugMessageUi] request.
#[derive(Debug)]
pub enum DebugMessageResponse {
    MemoryResponse(<MemoryEditorView as DebugView>::RequestedData),
    CpuResponse(<CpuStateView as DebugView>::RequestedData),
    PaletteResponse(<PaletteView as DebugView>::RequestedData),
}
