use std::fmt::Debug;

use egui::{Context, Ui};
use serde::{Deserialize, Serialize};

use crate::gui::debug::cpu_state_view::CpuStateView;
use crate::BoolUtils;
use grba_core::emulator::debug::DebugEmulator;

use crate::gui::debug::memory_view::MemoryEditorView;
use crate::gui::debug::messages::{DebugMessageResponse, DebugMessageUi};
use crate::gui::debug::palette_view::PaletteView;

mod colors;
pub mod cpu_state_view;
pub mod memory_view;
pub mod messages;
pub mod palette_view;

pub trait DebugView {
    /// The name of the debug view, used for the menu title.
    const NAME: &'static str;

    /// The type of the data that should be gathered from the emulator, and which can subsequently be used to render the view.
    type RequestedData: Debug;
    /// Additional information to inform the [Self::prepare_frame] call what to gather.
    type RequestInformation: Debug;
    /// Any changes that should be actuated on the emulator.
    type EmuUpdate: Debug;

    /// Prepare the Debug View for use.
    ///
    /// This is only called on the emulator thread, and thus does not take `self`.
    /// The created [Self::RequestedData] is passed to [Self::draw] after being transferred over a channel.
    fn prepare_frame(emu: &mut DebugEmulator, request_information: Self::RequestInformation) -> Self::RequestedData;

    /// Update the emulator state based on the [Self::EmuUpdate] data that was returned from the last [Self::draw] call.
    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate);

    /// Provide [Self::RequestInformation] for the next frame's [Self::prepare_frame] call.
    fn request_information(&mut self) -> Self::RequestInformation;

    /// Take the data prepared in [Self::prepare_frame] and update internal state to the provided [Self::RequestedData]
    fn update_requested_data(&mut self, data: Self::RequestedData);

    /// Draw the [DebugView] UI, with data gathered during [Self:":prepare_frame].
    ///
    /// Note that there is a one frame delay between [Self::prepare_frame] and [Self::draw].
    ///
    /// # Returns
    ///
    /// The [Self::EmuUpdate] data that should be passed to [Self::update_emu]. If no state update should be made, return `None`.
    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate>;
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct UiState {
    pub memory_open: bool,
    pub cpu_open: bool,
    pub palette_open: bool,
}

pub struct DebugViewManager {
    memory: MemoryEditorView,
    cpu_viewer: CpuStateView,
    palette_viewer: PaletteView,

    pub state: UiState,
}

impl DebugViewManager {
    /// Gather all debug information for a message.
    pub fn handle_ui_request_message(emu: &mut DebugEmulator, msg: DebugMessageUi) -> DebugMessageResponse {
        match msg {
            DebugMessageUi::MemoryRequest(request, update) => {
                // Update any memory that needs to be updated
                if let Some(update) = update {
                    MemoryEditorView::update_emu(emu, update);
                }

                // Fill memory response with the requested memory
                let result = MemoryEditorView::prepare_frame(emu, request);

                DebugMessageResponse::MemoryResponse(result)
            }
            DebugMessageUi::CpuRequest(request) => {
                let result = CpuStateView::prepare_frame(emu, request);

                DebugMessageResponse::CpuResponse(result)
            }
            DebugMessageUi::PaletteRequest(request) => {
                let result = PaletteView::prepare_frame(emu, request);

                DebugMessageResponse::PaletteResponse(result)
            }
        }
    }
}

impl DebugViewManager {
    pub fn new(ui_state: Option<UiState>) -> Self {
        Self {
            memory: MemoryEditorView::new(Default::default()),
            cpu_viewer: CpuStateView::new(),
            palette_viewer: PaletteView::new(),
            state: ui_state.unwrap_or_default(),
        }
    }

    /// Handle messages returned from the emulator thread, and update all internal state.
    pub fn handle_response_message(&mut self, msg: DebugMessageResponse) {
        match msg {
            DebugMessageResponse::MemoryResponse(data) => {
                self.memory.update_requested_data(data);
            }
            DebugMessageResponse::CpuResponse(data) => {
                self.cpu_viewer.update_requested_data(data);
            }
            DebugMessageResponse::PaletteResponse(data) => {
                self.palette_viewer.update_requested_data(data);
            }
        }
    }

    /// Draw menu buttons for enabling/disabling the debug views.
    pub fn draw_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            if ui
                .checkbox(&mut self.state.memory_open, MemoryEditorView::NAME)
                .clicked()
            {
                ui.close_menu();
            }

            if ui.checkbox(&mut self.state.cpu_open, CpuStateView::NAME).clicked() {
                ui.close_menu();
            }

            if ui.checkbox(&mut self.state.palette_open, PaletteView::NAME).clicked() {
                ui.close_menu();
            }
        });
    }

    /// Draw the debug views.
    ///
    /// # Returns
    ///
    /// The data requests from all enabled debug views.
    pub fn draw(&mut self, ctx: &Context) -> Vec<DebugMessageUi> {
        let mut result = Vec::new();

        if self.state.memory_open {
            let response = self.memory.draw(ctx, &mut self.state.memory_open);
            let request = self.memory.request_information();

            result.push(DebugMessageUi::MemoryRequest(request, response));
        }

        if self.state.cpu_open {
            let _ = self.cpu_viewer.draw(ctx, &mut self.state.cpu_open);
            let request = self.cpu_viewer.request_information();

            result.push(DebugMessageUi::CpuRequest(request));
        }

        if self.state.palette_open {
            let _ = self.palette_viewer.draw(ctx, &mut self.state.palette_open);
            let request = self.palette_viewer.request_information();

            result.push(DebugMessageUi::PaletteRequest(request));
        }

        result
    }
}
