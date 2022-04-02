use egui::{Color32, Context, Direction, Layout, RichText, Separator, TextStyle, Ui, Vec2};
use egui_memory_editor::MemoryEditor;
use itertools::Itertools;

use grba_core::emulator::cpu::registers::{Mode, Registers, PSR};
use grba_core::emulator::debug::DebugEmulator;

use crate::gui::debug::colors::{DARK_GREY, LIGHT_GREY};
use crate::gui::debug::memory_view::{MemContents, MemRequest, MemResponse, MemoryEditorView};
use crate::gui::debug::{colors, DebugView};

pub struct CpuStateView {
    cpu_state: CpuState,
    selected_banked: Mode,
}

#[derive(Debug, Default)]
pub struct CpuState {
    registers: Registers,
}

#[derive(Debug)]
pub struct CpuStateRequest;

impl CpuStateView {
    pub fn new() -> Self {
        Self {
            cpu_state: Default::default(),
            selected_banked: Mode::IRQ,
        }
    }
}

impl DebugView for CpuStateView {
    const NAME: &'static str = "CPU State";
    type RequestedData = CpuState;
    type RequestInformation = CpuStateRequest;
    type EmuUpdate = ();

    fn prepare_frame(emu: &mut DebugEmulator, _request_information: Self::RequestInformation) -> Self::RequestedData {
        CpuState {
            registers: emu.cpu().registers.clone(),
        }
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {}

    fn request_information(&mut self) -> Self::RequestInformation {
        CpuStateRequest
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.cpu_state = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        let state = &self.cpu_state;

        egui::containers::Window::new("ARM7 State")
            .resizable(true)
            .vscroll(true)
            .open(open)
            .show(ctx, |ui| {
                state.draw(ui);
            });

        None
    }
}

impl CpuState {
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
