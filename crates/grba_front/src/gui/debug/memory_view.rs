use crate::gui::debug::DebugView;
use egui::Context;
use egui_memory_editor::option_data::MemoryEditorOptions;
use egui_memory_editor::{Address, MemoryEditor};
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::MemoryAddress;
use std::ops::Range;

pub struct MemoryEditorView {
    egui_editor: MemoryEditor,
    mem_contents: MemContents,
    last_visible_address: Range<Address>,
}

#[derive(Debug)]
pub struct MemRequest {
    visible_address_range: Range<Address>,
}

#[derive(Debug)]
pub struct MemContents {
    visible_address_range: Range<Address>,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct MemResponse {
    data: Vec<(MemoryAddress, u8)>,
}

impl MemoryEditorView {
    pub fn new(options: MemoryEditorOptions) -> Self {
        let mem_contents = MemContents {
            visible_address_range: 0..0,
            data: vec![0; 0],
        };

        let egui_editor = MemoryEditor::new()
            .with_options(options)
            .with_address_range("IO", 0x0400_0000..0x0400_0804)
            .with_address_range("B-WRAM", 0x0200_0000..0x0204_0000)
            .with_address_range("C-WRAM", 0x0300_0000..0x0300_8000)
            .with_address_range("Palette", 0x0500_0000..0x0500_0400)
            .with_address_range("VRAM", 0x0600_0000..0x0601_8000)
            .with_address_range("OAM", 0x0700_0000..0x0700_0400)
            .with_address_range("ROM", 0x0800_0000..0x0A00_0000)
            .with_address_range("SRAM", 0x0E00_0000..0x0E01_0000);

        Self {
            egui_editor,
            mem_contents,
            last_visible_address: 0..0,
        }
    }
}

impl DebugView for MemoryEditorView {
    const NAME: &'static str = "Memory Editor";
    type RequestedData = MemContents;
    type RequestInformation = MemRequest;
    type EmuUpdate = MemResponse;

    fn prepare_frame(emu: &mut DebugEmulator, request_information: Self::RequestInformation) -> Self::RequestedData {
        let mut mem_contents = MemContents {
            visible_address_range: request_information.visible_address_range.clone(),
            data: Vec::with_capacity(request_information.visible_address_range.len()),
        };

        let (bus, cpu) = emu.bus_and_cpu();

        for i in request_information.visible_address_range {
            mem_contents.data.push(bus.read_dbg(i as u32, cpu));
        }

        mem_contents
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {
        let (bus, cpu) = emu.bus_and_cpu();

        for (address, value) in update.data {
            // TODO: Make a debug write function which ignores data bus shenanigans (like VRAM not being writable with u8)
            bus.write_dbg(address, value);
        }
    }

    fn request_information(&mut self) -> Self::RequestInformation {
        let mut range = self.last_visible_address.clone();
        range.start = range.start.saturating_sub(self.egui_editor.options.column_count * 6);
        // We subtract 6 rows worth to have a little more leeway if the user scrolls up
        MemRequest {
            visible_address_range: range,
        }
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.mem_contents = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        let mem_contents = &mut self.mem_contents;
        let mut update = MemResponse { data: Vec::new() };

        self.egui_editor.window_ui(
            ctx,
            open,
            mem_contents,
            |mem_contents, addr| {
                if mem_contents.visible_address_range.contains(&addr) {
                    let addr = addr - mem_contents.visible_address_range.start;
                    Some(mem_contents.data[addr])
                } else {
                    // If we have no data for the new range we'll just return 0xFF for now
                    None
                }
            },
            |mem_contents, addr, value| {
                if mem_contents.visible_address_range.contains(&addr) {
                    let addr = addr - mem_contents.visible_address_range.start;
                    // Pre-emptively update the data to prevent flickering
                    mem_contents.data[addr] = value;
                }

                update.data.push((addr as MemoryAddress, value));
            },
        );

        self.last_visible_address = self.egui_editor.visible_range().clone();

        if !update.data.is_empty() {
            Some(update)
        } else {
            None
        }
    }
}
