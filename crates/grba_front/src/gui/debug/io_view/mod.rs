use std::ops::Range;

use egui::{Context, Ui};
use itertools::Itertools;

use grba_core::emulator::cpu::registers::Registers;
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::DebugView;

mod io_utils;
mod registers;

pub struct IoView {
    state: IoState,
    frame_data: IoFrameData,
}

/// Private data for Egui display
struct IoFrameData {
    selected_reg: usize,
    search_str: String,
    implemented_only: bool,
}

#[derive(Debug, Default)]
pub struct IoState {
    registers: Registers,
    visible_address_range: Range<MemoryAddress>,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct IoStateRequest {
    visible_address_range: Range<MemoryAddress>,
}

#[derive(Debug)]
pub struct IoStateResponse {
    data: Vec<(MemoryAddress, u8)>,
}

impl IoView {
    pub fn new() -> Self {
        let frame_data = IoFrameData {
            selected_reg: 0,
            search_str: "".to_string(),
            implemented_only: true,
        };

        Self {
            state: Default::default(),
            frame_data,
        }
    }
}

impl DebugView for IoView {
    const NAME: &'static str = "IO Viewer";
    type RequestedData = IoState;
    type RequestInformation = IoStateRequest;
    type EmuUpdate = IoStateResponse;

    fn prepare_frame(emu: &mut DebugEmulator, request_information: Self::RequestInformation) -> Self::RequestedData {
        let mut data = Vec::with_capacity(request_information.visible_address_range.len());

        let (bus, cpu) = emu.bus_and_cpu();
        for addr in request_information.visible_address_range.clone() {
            let byte = bus.read_dbg(addr, cpu);
            data.push(byte);
        }

        IoState {
            registers: emu.cpu().registers.clone(),
            visible_address_range: request_information.visible_address_range,
            data,
        }
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {
        let (bus, _cpu) = emu.bus_and_cpu();

        for (address, value) in update.data {
            // TODO: Make a debug write function which ignores data bus shenanigans (like VRAM not being writable with u8)
            bus.write(address, value);
        }
    }

    fn request_information(&mut self) -> Self::RequestInformation {
        let frame_data = &self.frame_data;
        let selected_reg = &registers::IO_REGISTER_VIEWS[frame_data.selected_reg];
        let range = &selected_reg.address;

        // We take a larger range than the actual register to allow for adjacent registers to be selected without
        // a 1 frame flicker
        IoStateRequest {
            visible_address_range: range.start().saturating_sub(20)..range.end().saturating_add(20),
        }
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.state = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        egui::containers::Window::new("IO Viewer")
            .resizable(true)
            .vscroll(false)
            .open(open)
            .show(ctx, |ui| self.draw_ui(ui))?
            .inner?
    }
}

impl IoView {
    pub fn draw_ui(&mut self, ui: &mut Ui) -> Option<IoStateResponse> {
        egui::containers::panel::SidePanel::left("IO Select")
            .resizable(false)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().always_show_scroll(true).show(ui, |ui| {
                    let resp = egui::TextEdit::singleline(&mut self.frame_data.search_str)
                        .hint_text("Search...")
                        .show(ui);

                    if resp.response.clicked() {
                        self.frame_data.search_str.clear();
                    }

                    ui.checkbox(&mut self.frame_data.implemented_only, "Impl Only");

                    ui.separator();

                    for (i, reg) in registers::IO_REGISTER_VIEWS
                        .iter()
                        .enumerate()
                        .filter(|(_, reg)| {
                            !self.frame_data.implemented_only
                                || reg.draw as usize != registers::unimplemented_view as usize
                        })
                        .filter(|(_, reg)| reg.name.contains(&self.frame_data.search_str))
                    {
                        ui.selectable_value(&mut self.frame_data.selected_reg, i, reg.name);
                    }
                })
            });

        let Self { frame_data, state } = &self;

        let selected_reg = &registers::IO_REGISTER_VIEWS[frame_data.selected_reg];
        let data_range = selected_reg.address.clone();

        // Main panel
        let response = egui::ScrollArea::vertical()
            .always_show_scroll(true)
            .show(ui, |ui| {
                let data = if state.visible_address_range.contains(data_range.start())
                    && state.visible_address_range.contains(data_range.end())
                {
                    let start = (data_range.start() - state.visible_address_range.start) as usize;
                    &state.data[start..start + data_range.size_hint().0]
                } else {
                    // Some random stub data
                    &[0; 16][0..data_range.size_hint().0]
                };

                ui.label(selected_reg.name);

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(format!("Address: {:#010X}", data_range.start()));

                    ui.separator();

                    ui.label(format!("Value: {}", (selected_reg.format)(data)));
                });

                ui.separator();

                (selected_reg.draw)(ui, data)
            })
            .inner?;

        Some(IoStateResponse {
            data: response
                .into_iter()
                .zip(data_range)
                .map(|(response, address)| (address, response))
                .collect(),
        })
    }
}
