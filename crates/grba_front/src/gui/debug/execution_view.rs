use std::ops::Range;

use capstone::prelude::{BuildsCapstone, BuildsCapstoneSyntax};
use capstone::{arch, Capstone};
use egui::{Context, Direction, Layout, RichText, ScrollArea, Sense, Separator, TextStyle, Ui, Vec2};
use egui_memory_editor::Address;

use grba_core::emulator::cpu::registers::{Registers, State, PSR};
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::colors::{DARK_GREY, LIGHT_GREY};
use crate::gui::debug::{colors, DebugView};

pub struct CpuExecutionView {
    cpu_state: CpuState,
    last_visible_address: Range<Address>,
    break_points: Vec<MemoryAddress>,
    // Display
    capstone: Capstone,
    debug_enabled: bool,
    selected_address: Option<Address>,
    force_state: Option<State>,
}

#[derive(Debug, Default)]
pub struct CpuState {
    registers: Registers,
    visible_address_range: Range<Address>,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct CpuStateRequest {
    visible_address_range: Range<Address>,
}

impl CpuExecutionView {
    pub fn new() -> Self {
        let capstone = capstone::Capstone::new()
            .arm()
            .mode(capstone::arch::arm::ArchMode::Arm)
            .syntax(arch::arm::ArchSyntax::NoRegName)
            .detail(true)
            .build()
            .unwrap();

        Self {
            cpu_state: Default::default(),
            selected_address: None,
            last_visible_address: Default::default(),
            force_state: None,
            capstone,
            debug_enabled: false,
            break_points: vec![],
        }
    }
}

#[derive(Debug)]
pub enum CpuExecutionUpdate {
    StepInstruction,
    StepFrame,
    SetDebug(bool),
    SetBreakpoints(Vec<MemoryAddress>),
}

impl DebugView for CpuExecutionView {
    const NAME: &'static str = "CPU Execution";
    type RequestedData = CpuState;
    type RequestInformation = CpuStateRequest;
    type EmuUpdate = Vec<CpuExecutionUpdate>;

    fn prepare_frame(emu: &mut DebugEmulator, request_information: Self::RequestInformation) -> Self::RequestedData {
        let mut result = CpuState {
            registers: emu.cpu().registers.clone(),
            visible_address_range: request_information.visible_address_range.clone(),
            data: Vec::with_capacity(request_information.visible_address_range.len()),
        };

        let (bus, cpu) = emu.bus_and_cpu();

        for i in request_information.visible_address_range {
            result.data.push(bus.read(i as u32, cpu));
        }

        result
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {
        for update in update {
            match update {
                CpuExecutionUpdate::StepInstruction => {
                    let (vblank, breakpoint_hit) = emu.0.step_instruction_debug();

                    if breakpoint_hit {
                        println!("Breakpoint hit");
                    }
                }
                CpuExecutionUpdate::StepFrame => {
                    let breakpoint = emu.0.run_to_vblank_debug();

                    if breakpoint {
                        println!("Breakpoint reached");
                    }
                }
                CpuExecutionUpdate::SetDebug(value) => emu.0.options.debugging = value,
                CpuExecutionUpdate::SetBreakpoints(breakpoints) => emu.debug_info().breakpoints = breakpoints,
            };
        }
    }

    fn request_information(&mut self) -> Self::RequestInformation {
        CpuStateRequest {
            visible_address_range: self.last_visible_address.clone(),
        }
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.cpu_state = data;

        let state = self
            .force_state
            .unwrap_or_else(|| self.cpu_state.registers.cpsr.state());

        let current_mode = match state {
            State::Arm => capstone::Mode::Arm,
            State::Thumb => capstone::Mode::Thumb,
        };
        // Won't fail
        let _ = self.capstone.set_mode(current_mode);
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        let mut updates = Vec::new();

        egui::containers::Window::new("ARM7 Disassembly")
            .resizable(true)
            .vscroll(true)
            .open(open)
            .show(ctx, |ui| {
                self.draw_window_content(ui, &mut updates);
            });

        if updates.is_empty() {
            None
        } else {
            Some(updates)
        }
    }
}

impl CpuExecutionView {
    pub fn draw_window_content(&mut self, ui: &mut Ui, updates: &mut Vec<CpuExecutionUpdate>) {
        self.draw_actions(ui, updates);

        ui.separator();

        ui.style_mut().override_text_style = Some(TextStyle::Monospace);

        let active_state = self
            .force_state
            .unwrap_or_else(|| self.cpu_state.registers.cpsr.state());
        let addr_mult = match active_state {
            State::Arm => 4,
            State::Thumb => 2,
        };

        let line_height = self.get_line_height(ui);
        let address_characters = 8usize;
        let pc = self.cpu_state.registers.next_pc() as usize;
        let address_space = pc.saturating_sub(40)..pc + 0x500;
        let max_lines = address_space.len();

        let mut scroll = ScrollArea::vertical()
            .id_source("execution_view")
            .max_height(f32::INFINITY)
            .auto_shrink([false, true]);

        scroll.show_rows(ui, line_height, max_lines, |ui, line_range| {
            // Persist the visible range for future queries.
            let start_address_range = address_space.start + (line_range.start * addr_mult);
            let end_address_range = address_space.start + (line_range.end * addr_mult);
            self.last_visible_address = start_address_range..end_address_range;

            egui::Grid::new("mem_edit_grid")
                .striped(true)
                .spacing(Vec2::new(15.0, ui.style().spacing.item_spacing.y))
                .show(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.style_mut().spacing.item_spacing.x = 3.0;

                    for start_row in line_range.clone() {
                        let start_address = address_space.start + (start_row * addr_mult);
                        let highlight_in_range =
                            matches!(self.selected_address, Some(address) if address == start_address);
                        let is_pc = start_address == pc;
                        let is_breakpoint = self.break_points.contains(&(start_address as MemoryAddress));

                        let mut start_text = RichText::new(format!("0x{:01$X}:", start_address, address_characters))
                            .color(if is_pc {
                                colors::DARK_RED
                            } else if highlight_in_range {
                                colors::HIGHLIGHT
                            } else {
                                colors::DARK_PURPLE
                            });

                        if is_breakpoint {
                            start_text = start_text.background_color(colors::LIGHT_RED);
                        }

                        let response = ui
                            .add(egui::Label::new(start_text).sense(Sense::click()))
                            .on_hover_text("Click to select, right-click to set breakpoint");
                        // Select the address
                        if response.clicked() {
                            if matches!(self.selected_address, Some(address) if address == start_address) {
                                self.selected_address = None;
                            } else {
                                self.selected_address = Some(start_address);
                            }
                        }
                        // Set breakpoint
                        if response.secondary_clicked() {
                            if self.break_points.contains(&(start_address as MemoryAddress)) {
                                self.break_points
                                    .retain(|address| *address != start_address as MemoryAddress);
                            } else {
                                self.break_points.push(start_address as MemoryAddress);
                            }

                            updates.push(CpuExecutionUpdate::SetBreakpoints(self.break_points.clone()));
                        }

                        self.draw_instruction(ui, active_state, start_address, &address_space);

                        ui.end_row();
                    }
                });
        });
    }

    fn draw_actions(&mut self, ui: &mut Ui, updates: &mut Vec<CpuExecutionUpdate>) {
        ui.horizontal(|ui| {
            if ui.button("Step").clicked() {
                updates.push(CpuExecutionUpdate::StepInstruction);
            }

            if ui.button("Step Frame").clicked() {
                updates.push(CpuExecutionUpdate::StepFrame);
            }

            let possible_values = ["Arm".to_string(), "Thumb".to_string(), "None".to_string()];

            let mut current_state = self
                .force_state
                .map(|mode| format!("{:?}", mode))
                .unwrap_or_else(|| "None".to_string());

            egui::ComboBox::new("ForceModeExecution", "Force Mode")
                .selected_text(&current_state)
                .show_ui(ui, |ui| {
                    for value in possible_values {
                        ui.selectable_value(&mut current_state, value.clone(), value);
                    }
                })
                .response
                .on_hover_text("Will force the disassembly to interpret the data as either `Arm` or `Thumb`");

            if current_state == "Arm" {
                self.force_state = Some(State::Arm);
            } else if current_state == "Thumb" {
                self.force_state = Some(State::Thumb);
            } else {
                self.force_state = None;
            }

            if ui
                .checkbox(&mut self.debug_enabled, "Debug Enabled")
                .on_hover_text("If enabled will allow breakpoints to work")
                .clicked()
            {
                updates.push(CpuExecutionUpdate::SetDebug(self.debug_enabled));
            }
        });
    }

    fn draw_instruction(&self, ui: &mut Ui, state: State, address: usize, address_space: &Range<usize>) {
        let vis_range = &self.cpu_state.visible_address_range;

        if vis_range.contains(&address) && vis_range.contains(&(address + 4)) {
            let addr = address - vis_range.start;
            let data = match state {
                State::Arm => &self.cpu_state.data[addr..addr.saturating_add(4)],
                State::Thumb => &self.cpu_state.data[addr..addr.saturating_add(2)],
            };

            let disassembled = self.capstone.disasm_all(data, address as u64).unwrap();

            match state {
                State::Arm => ui.colored_label(
                    colors::LIGHT_GREY,
                    format!("{:08X}", u32::from_le_bytes(data.try_into().unwrap())),
                ),
                State::Thumb => ui.colored_label(
                    colors::LIGHT_GREY,
                    format!("{:04X}", u16::from_le_bytes(data.try_into().unwrap())),
                ),
            };

            if let Some(instr) = disassembled.get(0) {
                let text = RichText::new(format!("{} {}", instr.mnemonic().unwrap(), instr.op_str().unwrap()));

                ui.label(text);
            } else {
                ui.label("ERROR");
            }
        }
    }

    /// Return the line height for the current provided `Ui` and selected `TextStyle`s
    fn get_line_height(&self, ui: &mut Ui) -> f32 {
        let text_style = ui.text_style_height(&TextStyle::Monospace);
        text_style
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
