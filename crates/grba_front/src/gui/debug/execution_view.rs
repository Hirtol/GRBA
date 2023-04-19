use std::ops::Range;

use capstone::prelude::{BuildsCapstone, BuildsCapstoneSyntax};
use capstone::{arch, Capstone};
use egui::{Context, Key, RichText, ScrollArea, Sense, TextStyle, Ui, Vec2};
use egui_memory_editor::Address;

use grba_core::emulator::cpu::registers::{Registers, State};
use grba_core::emulator::debug::{Breakpoint, DebugEmulator};
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::{colors, DebugView};

pub struct CpuExecutionView {
    cpu_state: CpuState,
    last_visible_address: Range<Address>,
    break_points: Vec<MemoryAddress>,
    cycle_break: Option<u64>,
    // Display
    capstone: Capstone,
    debug_enabled: bool,
    selected_address: Option<Address>,
    force_state: Option<State>,
    frame_state: FrameState,
}

pub struct FrameState {
    break_cycle_input: String,
    add_breakpoint_input: String,
    jump_to_pc: bool,
}

#[derive(Debug, Default)]
pub struct CpuState {
    registers: Registers,
    visible_address_range: Range<Address>,
    data: Vec<u8>,
    last_hit_breakpoint: Option<Breakpoint>,
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
            frame_state: FrameState {
                break_cycle_input: String::new(),
                add_breakpoint_input: String::new(),
                jump_to_pc: false,
            },
            cycle_break: None,
        }
    }
}

#[derive(Debug)]
pub enum CpuExecutionUpdate {
    StepInstruction,
    StepFrame,
    SetDebug(bool),
    SetBreakpoints(Vec<MemoryAddress>),
    /// Set the break cycle at the `u64`th clock cycle if the `bool` is `false`.
    ///
    /// Otherwise interpret the `u64` as a relative clock.
    SetBreakCycle(Option<(bool, u64)>),
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
            last_hit_breakpoint: emu.debug_info().last_hit_breakpoint.clone(),
        };

        let (bus, cpu) = emu.bus_and_cpu();

        for i in request_information.visible_address_range {
            result.data.push(bus.read_dbg(i as u32, cpu));
        }

        result
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {
        for update in update {
            match update {
                CpuExecutionUpdate::StepInstruction => {
                    let (_, breakpoint_hit) = emu.0.step_instruction_debug();

                    if breakpoint_hit {
                        log::debug!("Breakpoint hit");
                    }
                }
                CpuExecutionUpdate::StepFrame => {
                    let breakpoint = emu.0.run_to_vblank_debug();

                    if breakpoint {
                        log::debug!("Breakpoint reached");
                    }
                }
                CpuExecutionUpdate::SetDebug(value) => emu.0.options.debugging = value,
                CpuExecutionUpdate::SetBreakpoints(breakpoints) => emu.debug_info().breakpoints = breakpoints,
                CpuExecutionUpdate::SetBreakCycle(Some((is_relative, cycle))) => {
                    if is_relative {
                        emu.debug_info().break_at_cycle = Some(emu.bus().scheduler.current_time.0 + cycle);
                    } else {
                        emu.debug_info().break_at_cycle = Some(cycle);
                    }
                }
                CpuExecutionUpdate::SetBreakCycle(None) => {
                    emu.debug_info().break_at_cycle = None;
                    if matches!(emu.debug_info().last_hit_breakpoint, Some(Breakpoint::Cycle(_))) {
                        emu.debug_info().last_hit_breakpoint = None;
                    }
                }
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
        self.draw_breakpoints_info(ui, updates);

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

        if self.frame_state.jump_to_pc {
            self.frame_state.jump_to_pc = false;
            scroll = scroll.vertical_scroll_offset(line_height * 10.);
        }

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
                            if is_pc {
                                start_text = start_text.color(colors::LIGHT_GREY)
                            }
                        }

                        let response = ui
                            .add(egui::Label::new(start_text).sense(Sense::click()))
                            .on_hover_text("Click to select, right-click to set breakpoint");

                        // Select the address
                        if response.clicked() {
                            if response.double_clicked() {
                                ui.output_mut(|o| {
                                    o.copied_text = format!("0x{:01$X}", start_address, address_characters);
                                })
                            }
                            if matches!(self.selected_address, Some(address) if address == start_address) {
                                self.selected_address = None;
                            } else {
                                self.selected_address = Some(start_address);
                            }
                        }
                        // Set breakpoint
                        if response.secondary_clicked() {
                            if self.break_points.contains(&(start_address as MemoryAddress)) {
                                self.delete_breakpoint(start_address as MemoryAddress);
                            } else {
                                self.break_points.push(start_address as MemoryAddress);
                            }

                            updates.push(CpuExecutionUpdate::SetBreakpoints(self.break_points.clone()));
                        }

                        self.draw_instruction(ui, active_state, start_address);

                        ui.end_row();
                    }
                });
        });
    }

    fn draw_breakpoints_info(&mut self, ui: &mut Ui, updates: &mut Vec<CpuExecutionUpdate>) {
        egui::containers::panel::SidePanel::left("Breakpoints")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().always_show_scroll(true).show(ui, |ui| {
                    let resp = egui::TextEdit::singleline(&mut self.frame_state.add_breakpoint_input)
                        .hint_text("Add Breakpoint")
                        .show(ui);

                    if resp.response.lost_focus() && ui.input(|ui| ui.key_pressed(Key::Enter)) {
                        let trimmed = self
                            .frame_state
                            .add_breakpoint_input
                            .strip_prefix("0x")
                            .unwrap_or(&self.frame_state.add_breakpoint_input);
                        let address = Address::from_str_radix(trimmed, 16).ok();

                        if let Some(address) = address {
                            self.break_points.push(address as MemoryAddress);
                        } else {
                            log::warn!(
                                "Tried to enter invalid address: `{}`",
                                self.frame_state.add_breakpoint_input
                            );
                        }

                        updates.push(CpuExecutionUpdate::SetBreakpoints(self.break_points.clone()));
                    }

                    if resp.response.clicked() {
                        self.frame_state.add_breakpoint_input.clear();
                    }

                    if let Some((mode, cycle)) = super::utils::text_edit_uint(
                        ui,
                        &mut self.frame_state.break_cycle_input,
                        "Cycle Break",
                        "Set the breakpoint at the given cycle.\nUse #{CYCLE} for a relative offset",
                        10,
                    ) {
                        // Doesn't account for relative... yeah...
                        self.cycle_break = Some(cycle);
                        updates.push(CpuExecutionUpdate::SetBreakCycle(Some((mode.is_relative(), cycle))));
                    }

                    ui.separator();

                    if let Some(cycle) = self.cycle_break {
                        let mut text = RichText::new(format!("Cycle({cycle})"));

                        if matches!(&self.cpu_state.last_hit_breakpoint, Some(Breakpoint::Cycle(_))) {
                            text = text.color(colors::DARK_RED);
                        }

                        ui.horizontal(|ui| {
                            ui.label(text);
                            if ui.button("🗑").clicked() {
                                self.cycle_break = None;
                                updates.push(CpuExecutionUpdate::SetBreakCycle(None));
                            }
                        });
                    }

                    let mut to_delete = None;

                    for (i, addr) in self.break_points.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{:#X}", addr));
                            if ui.button("🗑").clicked() {
                                to_delete = Some(*addr);
                            }
                        });
                    }

                    if let Some(delete) = to_delete {
                        self.delete_breakpoint(delete);
                        updates.push(CpuExecutionUpdate::SetBreakpoints(self.break_points.clone()));
                    }

                    ui.shrink_width_to_current();
                })
            });
    }

    fn draw_actions(&mut self, ui: &mut Ui, updates: &mut Vec<CpuExecutionUpdate>) {
        ui.horizontal(|ui| {
            if ui
                .button("Jump PC")
                .on_hover_text("Jump the cursor back to the current PC")
                .clicked()
            {
                self.frame_state.jump_to_pc = true
            }

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
                .checkbox(&mut self.debug_enabled, "Debug")
                .on_hover_text("If enabled will allow breakpoints to work")
                .clicked()
            {
                updates.push(CpuExecutionUpdate::SetDebug(self.debug_enabled));
            }
        });
    }

    fn draw_instruction(&self, ui: &mut Ui, state: State, address: usize) {
        let vis_range = &self.cpu_state.visible_address_range;

        if vis_range.contains(&address) && vis_range.contains(&(address + 4)) {
            let addr = address - vis_range.start;
            // We take 4 bytes, even in Thumb mode (because we want to disassemble the `bl` instruction correctly)
            let data = &self.cpu_state.data[addr..addr.saturating_add(4)];

            let disassembled = self.capstone.disasm_all(data, address as u64).unwrap();

            match state {
                State::Arm => ui.colored_label(
                    colors::LIGHT_GREY,
                    format!("{:08X}", u32::from_le_bytes(data.try_into().unwrap())),
                ),
                State::Thumb => ui.colored_label(
                    colors::LIGHT_GREY,
                    format!("{:04X}", u16::from_le_bytes(data[0..2].try_into().unwrap())),
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
        ui.text_style_height(&TextStyle::Monospace)
    }

    fn delete_breakpoint(&mut self, address: MemoryAddress) {
        self.break_points.retain(|item| *item != address)
    }
}
