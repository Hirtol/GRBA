use std::fmt::Debug;

use egui::{Context, Ui};
use serde::{Deserialize, Serialize};

use crate::gui::debug::cpu_state_view::CpuStateView;
use crate::gui::debug::emu_state::EmuStateView;
use crate::gui::debug::execution_view::CpuExecutionView;
use crate::gui::debug::io_view::IoView;
use grba_core::emulator::debug::DebugEmulator;

use crate::gui::debug::memory_view::MemoryEditorView;
use crate::gui::debug::messages::{DebugMessageResponse, DebugMessageUi};
use crate::gui::debug::palette_view::PaletteView;

mod colors;
pub mod cpu_state_view;
pub mod emu_state;
pub mod execution_view;
pub mod io_view;
pub mod memory_view;
pub mod messages;
pub mod palette_view;
mod utils;

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

    /// Draw the [DebugView] UI, with data gathered during [Self::prepare_frame].
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
    pub emu_state_open: bool,
    pub palette_open: bool,
    pub cpu_execute_open: bool,
    pub io_open: bool,
}

pub struct DebugViewManager {
    memory: MemoryEditorView,
    cpu_viewer: CpuStateView,
    emu_viewer: EmuStateView,
    palette_viewer: PaletteView,
    cpu_execution: CpuExecutionView,
    io_viewer: IoView,

    pub state: UiState,
}

impl DebugViewManager {
    /// Gather all debug information for a message.
    ///
    /// # Returns
    /// A potential response, as well as a boolean flag indicating whether a new frame should be dispatched.
    pub fn handle_ui_request_message(emu: &mut DebugEmulator, msg: DebugMessageUi) -> (DebugMessageResponse, bool) {
        match msg {
            DebugMessageUi::MemoryRequest(request, update) => {
                // Update any memory that needs to be updated
                if let Some(update) = update {
                    MemoryEditorView::update_emu(emu, update);
                }

                // Fill memory response with the requested memory
                let result = MemoryEditorView::prepare_frame(emu, request);

                (DebugMessageResponse::MemoryResponse(result), false)
            }
            DebugMessageUi::CpuRequest(request) => {
                let result = CpuStateView::prepare_frame(emu, request);

                (DebugMessageResponse::CpuResponse(result), false)
            }
            DebugMessageUi::EmuRequest(request) => {
                let result = EmuStateView::prepare_frame(emu, request);

                (DebugMessageResponse::EmuResponse(result), false)
            }
            DebugMessageUi::PaletteRequest(request) => {
                let result = PaletteView::prepare_frame(emu, request);

                (DebugMessageResponse::PaletteResponse(result), false)
            }
            DebugMessageUi::CpuExecuteRequest(request, update) => {
                // Update any memory that needs to be updated
                if let Some(update) = update {
                    CpuExecutionView::update_emu(emu, update);
                }

                // Fill memory response with the requested memory
                let result = CpuExecutionView::prepare_frame(emu, request);

                (DebugMessageResponse::CpuExecuteResponse(result), true)
            }
            DebugMessageUi::IoRequest(request, update) => {
                if let Some(update) = update {
                    IoView::update_emu(emu, update);
                }

                let result = IoView::prepare_frame(emu, request);

                (DebugMessageResponse::IoResponse(result), false)
            }
        }
    }
}

impl DebugViewManager {
    pub fn new(ui_state: Option<UiState>) -> Self {
        Self {
            memory: MemoryEditorView::new(Default::default()),
            cpu_viewer: CpuStateView::new(),
            emu_viewer: EmuStateView::new(),
            palette_viewer: PaletteView::new(),
            cpu_execution: CpuExecutionView::new(),
            io_viewer: IoView::new(),
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
            DebugMessageResponse::EmuResponse(data) => self.emu_viewer.update_requested_data(data),
            DebugMessageResponse::PaletteResponse(data) => {
                self.palette_viewer.update_requested_data(data);
            }
            DebugMessageResponse::CpuExecuteResponse(data) => {
                self.cpu_execution.update_requested_data(data);
            }
            DebugMessageResponse::IoResponse(data) => {
                self.io_viewer.update_requested_data(data);
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

            if ui.checkbox(&mut self.state.io_open, IoView::NAME).clicked() {
                ui.close_menu();
            }

            if ui.checkbox(&mut self.state.cpu_open, CpuStateView::NAME).clicked() {
                ui.close_menu();
            }

            if ui
                .checkbox(&mut self.state.emu_state_open, EmuStateView::NAME)
                .clicked()
            {
                ui.close_menu();
            }

            if ui
                .checkbox(&mut self.state.cpu_execute_open, CpuExecutionView::NAME)
                .clicked()
            {
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

        if self.state.io_open {
            let response = self.io_viewer.draw(ctx, &mut self.state.io_open);
            let request = self.io_viewer.request_information();

            result.push(DebugMessageUi::IoRequest(request, response));
        }

        if self.state.cpu_open {
            let _ = self.cpu_viewer.draw(ctx, &mut self.state.cpu_open);
            let request = self.cpu_viewer.request_information();

            result.push(DebugMessageUi::CpuRequest(request));
        }

        if self.state.emu_state_open {
            let _ = self.emu_viewer.draw(ctx, &mut self.state.emu_state_open);
            let request = self.emu_viewer.request_information();

            result.push(DebugMessageUi::EmuRequest(request));
        }

        if self.state.cpu_execute_open {
            let response = self.cpu_execution.draw(ctx, &mut self.state.cpu_execute_open);
            let request = self.cpu_execution.request_information();

            result.push(DebugMessageUi::CpuExecuteRequest(request, response));
        }

        if self.state.palette_open {
            let _ = self.palette_viewer.draw(ctx, &mut self.state.palette_open);
            let request = self.palette_viewer.request_information();

            result.push(DebugMessageUi::PaletteRequest(request));
        }

        result
    }
}
