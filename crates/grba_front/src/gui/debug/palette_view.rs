use egui::epaint::RectShape;
use egui::{
    Align, Color32, Context, Direction, Layout, Response, RichText, Rounding, Sense, Separator, Stroke, TextStyle, Ui,
    Vec2, Widget,
};
use egui_memory_editor::MemoryEditor;
use itertools::Itertools;

use grba_core::emulator::cpu::registers::{Mode, Registers, PSR};
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::ppu::Palette;

use crate::gui::debug::colors::{DARK_GREY, LIGHT_GREY};
use crate::gui::debug::cpu_state_view::{CpuState, CpuStateRequest, CpuStateView};
use crate::gui::debug::memory_view::{MemContents, MemRequest, MemResponse, MemoryEditorView};
use crate::gui::debug::{colors, DebugView};

pub struct PaletteView {
    palettes: PaletteState,
    selected_background: bool,
    selected_index: usize,
}

#[derive(Debug)]
pub struct PaletteState {
    palettes: [Palette; 512],
}

#[derive(Debug)]
pub struct PaletteStateRequest;

impl PaletteView {
    pub fn new() -> Self {
        Self {
            palettes: PaletteState {
                palettes: [Palette::default(); 512],
            },
            selected_background: true,
            selected_index: 0,
        }
    }
}

impl DebugView for PaletteView {
    const NAME: &'static str = "Palettes";
    type RequestedData = PaletteState;
    type RequestInformation = PaletteStateRequest;
    type EmuUpdate = ();

    fn prepare_frame(emu: &mut DebugEmulator, _request_information: Self::RequestInformation) -> Self::RequestedData {
        PaletteState {
            palettes: emu.bus().ppu.palette_cache().cache().clone(),
        }
    }

    fn update_emu(emu: &mut DebugEmulator, update: Self::EmuUpdate) {}

    fn request_information(&mut self) -> Self::RequestInformation {
        PaletteStateRequest
    }

    fn update_requested_data(&mut self, data: Self::RequestedData) {
        self.palettes = data;
    }

    fn draw(&mut self, ctx: &Context, open: &mut bool) -> Option<Self::EmuUpdate> {
        egui::containers::Window::new(Self::NAME)
            .resizable(false)
            .vscroll(false)
            .open(open)
            .show(ctx, |ui| {
                ui.style_mut().override_text_style = Some(TextStyle::Monospace);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Background:");
                        if let Some(idx) = self.palettes.render_palette(ui, true) {
                            self.selected_index = idx;
                        }
                    });

                    ui.add(egui::Separator::default().vertical());

                    ui.vertical(|ui| {
                        ui.label("Objects:");
                        if let Some(idx) = self.palettes.render_palette(ui, false) {
                            self.selected_index = idx;
                        }
                    });
                });

                ui.separator();

                ui.label("Selection:");

                ui.horizontal(|ui| {
                    egui::Grid::new("selection-grid").show(ui, |ui| {
                        ui.colored_label(colors::DARK_PURPLE, "Red:");
                        ui.label(format!("{:#04X}", self.palettes.palettes[self.selected_index].red));

                        ui.end_row();

                        ui.colored_label(colors::DARK_PURPLE, "Green:");
                        ui.label(format!("{:#04X}", self.palettes.palettes[self.selected_index].green));

                        ui.end_row();

                        ui.colored_label(colors::DARK_PURPLE, "Blue:");
                        ui.label(format!("{:#04X}", self.palettes.palettes[self.selected_index].blue));
                        ui.end_row();
                    });

                    ui.add(Separator::default().vertical());

                    egui::Grid::new("selection-grid").show(ui, |ui| {
                        ui.colored_label(colors::DARK_PURPLE, "Index:");
                        ui.label(format!("{:#05X} ({:03})", self.selected_index, self.selected_index));

                        ui.end_row();
                    });

                    ui.add(Separator::default().vertical());
                    PaletteWidget::new(&self.palettes.palettes[self.selected_index], Vec2::new(50.0, 50.0)).ui(ui);
                });
            });

        None
    }
}

impl PaletteState {
    fn render_palette(&self, ui: &mut Ui, background: bool) -> Option<usize> {
        let start_index = if background { 0 } else { 256 };
        let end_index_exclusive = start_index + 256;
        let palettes_to_draw = &self.palettes[start_index..end_index_exclusive];
        let mut selected_index = None;

        ui.vertical(|ui| {
            ui.style_mut().spacing.item_spacing = Vec2::new(4.0, 1.0);

            for palette_chunk in &palettes_to_draw.iter().enumerate().chunks(16) {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.item_spacing = Vec2::new(4.0, 1.0);
                    ui.set_max_height(6.0);
                    for (idx, palette) in palette_chunk {
                        let response = PaletteWidget::new(palette, Vec2::new(8.0, 8.0)).ui(ui);

                        if response.clicked() {
                            selected_index = Some(idx + start_index);
                        }

                        if response.hovered() {
                            response.on_hover_text(format!("Palette Index: {:#05X}", idx));
                        }
                    }
                });
            }
        });

        selected_index
    }
}

pub struct PaletteWidget<'a> {
    pub palette: &'a Palette,
    pub desired_size: Vec2,
}

impl<'a> PaletteWidget<'a> {
    pub fn new(palette: &'a Palette, size: Vec2) -> Self {
        Self {
            palette,
            desired_size: size,
        }
    }
}

impl<'a> Widget for PaletteWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_at_least(self.desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let color = egui::color::Color32::from_rgb(self.palette.red, self.palette.green, self.palette.blue);

            ui.painter().add(RectShape {
                rect,
                rounding: Rounding::none(),
                fill: color,
                stroke: Stroke::new(3.0, color.to_opaque()),
            });
        }

        response
    }
}
