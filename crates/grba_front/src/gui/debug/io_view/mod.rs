use std::ops::Range;

use egui::{Context, Direction, Layout, RichText, Separator, TextStyle, Ui, Vec2};
use itertools::Itertools;

use grba_core::emulator::cpu::registers::{Mode, Registers, PSR};
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::colors::{DARK_GREY, LIGHT_GREY};
use crate::gui::debug::io_view::registers::ViewableRegister;
use crate::gui::debug::memory_view::{MemContents, MemRequest, MemResponse, MemoryEditorView};
use crate::gui::debug::{colors, DebugView};

mod registers;

pub struct IoView {
    state: IoState,
    frame_data: IoFrameData,
}

/// Private data for Egui display
struct IoFrameData {
    selected_reg: usize,
    selected_instance: u32,
    selected_label_index: usize,
    registers: Vec<&'static dyn ViewableRegister>,
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
            selected_instance: 0,
            selected_label_index: 0,
            registers: registers::get_register_list(),
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
        let selected_reg = &frame_data.registers[frame_data.selected_reg];
        let range = selected_reg.get_address(frame_data.selected_instance);

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
                    let mut label_index = 0;

                    for (i, reg) in self.frame_data.registers.iter().enumerate() {
                        for instance in 0..reg.get_total_instances() {
                            let name = if reg.get_total_instances() == 1 {
                                reg.get_name().to_string()
                            } else {
                                format!("{}[{}]", reg.get_name(), instance)
                            };

                            let response =
                                ui.selectable_value(&mut self.frame_data.selected_label_index, label_index, name);

                            if response.changed() {
                                self.frame_data.selected_reg = i;
                                self.frame_data.selected_instance = instance;
                            }

                            label_index += 1;
                        }
                    }
                })
            });

        let Self { frame_data, state } = &self;

        let selected_reg = &frame_data.registers[frame_data.selected_reg];
        let data_range = selected_reg.get_address(frame_data.selected_instance);

        let data = if state.visible_address_range.contains(data_range.start())
            && state.visible_address_range.contains(data_range.end())
        {
            &state.data
        } else {
            return None;
        };

        ui.label(selected_reg.get_name());

        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!("Address: {:#010X}", data_range.start()));

            ui.separator();

            ui.label(format!("Value: {}", selected_reg.get_current_value(data)));
        });

        ui.separator();

        let response = selected_reg.draw(ui, data)?;

        Some(IoStateResponse {
            data: response
                .into_iter()
                .zip(data_range)
                .map(|(response, address)| (address, response))
                .collect(),
        })
    }
}

impl IoState {
    pub fn draw(&self, ui: &mut Ui) {
        ui.style_mut().override_text_style = Some(TextStyle::Monospace);
        // Registers
        egui::Grid::new("CPU State Registers Grid")
            .striped(true)
            .max_col_width(60.)
            .show(ui, |ui| {
                for regs in &self.registers.general_purpose.into_iter().enumerate().chunks(4) {
                    ui.vertical(|ui| {
                        for (i, reg) in regs {
                            ui.horizontal(|ui| {
                                ui.horizontal(|ui| {
                                    // Ensure that r5 and r15 have their value text properly aligned.
                                    ui.set_width_range(30.0..=30.0);

                                    let text = RichText::new(format!("r{}:", i)).color(colors::DARK_PURPLE);
                                    ui.label(text);
                                });

                                let value = RichText::new(format!("{:08X}", reg)).background_color(colors::LIGHT_GREY);

                                ui.label(value);
                            });
                        }
                    });
                }
            });

        ui.separator();

        // CPSR
        ui.label("CPSR:");

        render_psr(ui, &self.registers.cpsr);

        ui.separator();

        // SPSR:

        ui.label("SPSR:");

        // Ignore the modes without SPSRs
        if matches!(self.registers.cpsr.mode(), Mode::User | Mode::System) {
            ui.label("None");
        } else {
            render_psr(ui, &self.registers.spsr);
        }
    }
}

fn render_psr(ui: &mut Ui, psr: &PSR) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 5.0;

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.sign() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "N");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.zero() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "Z");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.carry() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "C");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.overflow() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "V");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:020b}", psr.reserved() >> 8))
                .color(LIGHT_GREY)
                .size(10.0)
                .background_color(DARK_GREY);
            ui.label(text);
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.irq_disable() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "I");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.fiq_disable() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "F");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:b}", psr.state() as u8)).background_color(LIGHT_GREY);
            ui.label(text);

            ui.colored_label(colors::DARK_PURPLE, "T");
        });

        ui.vertical(|ui| {
            let text = RichText::new(format!("{:05b}", psr.mode() as u8)).background_color(LIGHT_GREY);
            let size = ui.label(text).rect.size();
            ui.allocate_ui(size, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(colors::DARK_PURPLE, "Mode");
                });
            });
        });
    });

    ui.horizontal(|ui| {
        ui.colored_label(colors::DARK_PURPLE, "State:");

        ui.label(format!("{:?}", psr.state()));

        ui.add(Separator::default().vertical());

        ui.colored_label(colors::DARK_PURPLE, "Mode:");

        ui.label(format!("{:?}", psr.mode()));
    });
}
