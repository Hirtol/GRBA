use egui::{Context, TextStyle, Ui};

use grba_core::emulator::debug::DebugEmulator;
use grba_core::scheduler::{EmuTime, Event};

use crate::gui::debug::DebugView;

pub struct EmuStateView {
    emu_state: EmuState,
}

#[derive(Debug, Default)]
pub struct EmuState {
    current_timestamp: EmuTime,
    scheduler_events: Vec<Event>,
}

#[derive(Debug)]
pub struct EmuStateRequest;

impl EmuStateView {
    pub fn new() -> Self {
        Self {
            emu_state: Default::default(),
        }
    }
}

impl DebugView for EmuStateView {
    const NAME: &'static str = "Emulator State";
    type RequestedData = EmuState;
    type RequestInformation = EmuStateRequest;
    type EmuUpdate = ();

    fn prepare_frame(emu: &mut DebugEmulator, _request_information: Self::RequestInformation) -> Self::RequestedData {
        EmuState {
            current_timestamp: emu.bus().scheduler.current_time,
            scheduler_events: emu.bus().scheduler.event_queue(),
        }
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {}

    fn request_information(&mut self) -> Self::RequestInformation {
        EmuStateRequest
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.emu_state = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        let state = &self.emu_state;

        egui::containers::Window::new("Emulator State")
            .resizable(true)
            .vscroll(true)
            .open(open)
            .show(ctx, |ui| {
                state.draw(ui);
            });

        None
    }
}

impl EmuState {
    pub fn draw(&self, ui: &mut Ui) {
        ui.style_mut().override_text_style = Some(TextStyle::Monospace);
        // Registers

        egui::Grid::new("Scheduler Data").striped(true).show(ui, |ui| {
            ui.label("Scheduler Time:");
            ui.label(format!("{}", self.current_timestamp.0));

            ui.end_row();

            ui.label("Events:");
            ui.vertical(|ui| {
                ui.style_mut().wrap = Some(false);
                for event in &self.scheduler_events {
                    ui.label(format!(
                        "{:?}({})",
                        event.tag,
                        event.timestamp.0 - self.current_timestamp.0
                    ));
                }
            });
        });

        ui.separator();
    }
}
