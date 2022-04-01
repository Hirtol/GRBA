use std::fmt::Debug;

use egui::{Context, Ui};

use crate::gui::debug::cpu_state::CpuStateView;
use grba_core::emulator::debug::DebugEmulator;

use crate::gui::debug::memory_view::MemoryEditorView;
use crate::gui::debug::messages::{DebugMessageResponse, DebugMessageUi};

mod colors;
pub mod cpu_state;
pub mod memory_view;
pub mod messages;

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

    /// Set whether the view should be enabled or not
    fn set_open(&mut self, open: bool);

    /// Whether the current view is open/enabled.
    ///
    /// If `false` then neither [Self::prepare_frame] or [Self::draw] will be called.
    fn is_open(&self) -> bool;

    /// Take the data prepared in [Self::prepare_frame] and update internal state to the provided [Self::RequestedData]
    fn update_requested_data(&mut self, data: Self::RequestedData);

    /// Draw the [DebugView] UI, with data gathered during [Self:":prepare_frame].
    ///
    /// Note that there is a one frame delay between [Self::prepare_frame] and [Self::draw].
    ///
    /// # Returns
    ///
    /// The [Self::EmuUpdate] data that should be passed to [Self::update_emu]. If no state update should be made, return `None`.
    fn draw(&mut self, ctx: &Context) -> Option<Self::EmuUpdate>;
}

pub struct DebugViewManager {
    memory: MemoryEditorView,
    cpu_viewer: CpuStateView,
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
        }
    }
}

impl DebugViewManager {
    pub fn new() -> Self {
        Self {
            memory: MemoryEditorView::new(Default::default()),
            cpu_viewer: CpuStateView::new(),
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
        }
    }

    pub fn draw_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            if ui
                .checkbox(&mut self.memory.is_open(), MemoryEditorView::NAME)
                .clicked()
            {
                self.memory.set_open(!self.memory.is_open());
                ui.close_menu();
            }

            if ui
                .checkbox(&mut self.cpu_viewer.is_open(), CpuStateView::NAME)
                .clicked()
            {
                self.cpu_viewer.set_open(!self.cpu_viewer.is_open());
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

        if self.memory.is_open() {
            let response = self.memory.draw(ctx);
            let request = self.memory.request_information();

            result.push(DebugMessageUi::MemoryRequest(request, response));
        }

        if self.cpu_viewer.is_open() {
            let _ = self.cpu_viewer.draw(ctx);
            let request = self.cpu_viewer.request_information();

            result.push(DebugMessageUi::CpuRequest(request));
        }

        result
    }
}
