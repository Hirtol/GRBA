use std::ops::Range;

use egui::{Context, Direction, Layout, RichText, Separator, TextStyle, Ui, Vec2};
use itertools::Itertools;

use grba_core::emulator::cpu::registers::{Mode, Registers, PSR};
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::colors::LIGHT_GREY;
use crate::gui::debug::memory_view::{MemContents, MemRequest, MemResponse, MemoryEditorView};
use crate::gui::debug::{colors, DebugView};

mod io_utils;
mod registers;

pub struct IoView {
    state: IoState,
    frame_data: IoFrameData,
}

/// Private data for Egui display
struct IoFrameData {
    selected_reg: usize,
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
        let frame_data = IoFrameData { selected_reg: 0 };

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
            let byte = bus.read(addr, cpu);
            data.push(byte);
        }

        IoState {
            registers: emu.cpu().registers.clone(),
            visible_address_range: request_information.visible_address_range,
            data,
        }
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {
        let (bus, cpu) = emu.bus_and_cpu();

        for (address, value) in update.data {
            // TODO: Make a debug write function which ignores data bus shenanigans (like VRAM not being writable with u8)
            bus.write(address, value);
        }
    }

    fn request_information(&mut self) -> Self::RequestInformation {
        let frame_data = &self.frame_data;
        let selected_reg = &registers::IO_REGISTER_VIEWS[frame_data.selected_reg];
        let range = &selected_reg.address;

        IoStateRequest {
            visible_address_range: *range.start()..range.end().saturating_add(1),
        }
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.state = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        egui::containers::Window::new("IO Viewer")
            .resizable(true)
            .vscroll(true)
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
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, reg) in registers::IO_REGISTER_VIEWS.iter().enumerate() {
                        ui.selectable_value(&mut self.frame_data.selected_reg, i, reg.name);
                    }
                })
            });

        let Self { frame_data, state } = &self;

        let selected_reg = &registers::IO_REGISTER_VIEWS[frame_data.selected_reg];
        let data_range = selected_reg.address.clone();

        let data = if state.visible_address_range.contains(data_range.start())
            && state.visible_address_range.contains(data_range.end())
        {
            &*state.data
        } else {
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

        let response = (selected_reg.draw)(ui, data)?;

        Some(IoStateResponse {
            data: response
                .into_iter()
                .zip(data_range)
                .map(|(response, address)| (address, response))
                .collect(),
        })
    }
}
