use egui::mutex::RwLockWriteGuard;
use egui::{ClippedPrimitive, Context, Memory, TexturesDelta};
use egui_wgpu_backend::{BackendError, RenderPass, ScreenDescriptor};
use pixels::{wgpu, PixelsContext};
use serde::{Deserialize, Serialize};
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::runner::messages::EmulatorMessage;
pub use debug::messages::{DebugMessageResponse, DebugMessageUi};
pub use debug::DebugViewManager;

mod debug;

/// Manages all state required for rendering egui over `Pixels`.
pub struct EguiFramework {
    // State for egui.
    egui_ctx: Context,
    // Egui Winit helper
    egui_state: egui_winit::State,
    // Egui WebGPU backend
    rpass: RenderPass,
    screen_descriptor: ScreenDescriptor,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,

    // State for the GUI
    pub gui: Gui,
}

impl EguiFramework {
    /// Create egui.
    pub fn new(
        width: u32,
        height: u32,
        scale_factor: f32,
        pixels: &pixels::Pixels,
        event_loop: &EventLoop<()>,
        ui_state: Option<AppUiState>,
    ) -> Self {
        let (egui_ctx, gui) = if let Some(mem) = ui_state {
            let egui_ctx = Context::default();
            egui_ctx.memory_mut(|writer| *writer = mem.egui);

            (egui_ctx, Gui::new(Some(mem.debug_ui)))
        } else {
            (Context::default(), Gui::new(None))
        };

        let max_texture_size = pixels.device().limits().max_texture_dimension_2d as usize;
        let mut egui_state = egui_winit::State::new(event_loop);
        egui_state.set_pixels_per_point(scale_factor);
        egui_state.set_max_texture_side(max_texture_size);

        let screen_descriptor = ScreenDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor,
        };
        let rpass = RenderPass::new(pixels.device(), pixels.render_texture_format(), 1);

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            rpass,
            paint_jobs: Vec::new(),
            textures: Default::default(),
            gui,
        }
    }

    /// Prepare egui.
    pub fn prepare(&mut self, window: &Window, state: &mut crate::State) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx, state);
        });

        self.textures.append(full_output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(full_output.shapes);
    }

    /// Render egui.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) -> Result<(), BackendError> {
        // Upload all resources to the GPU.
        self.rpass
            .add_textures(&context.device, &context.queue, &self.textures)?;

        self.rpass.update_buffers(
            &context.device,
            &context.queue,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Record all render passes. TODO: Make use of clear color here?
        self.rpass
            .execute(encoder, render_target, &self.paint_jobs, &self.screen_descriptor, None)?;

        let textures = std::mem::take(&mut self.textures);
        self.rpass.remove_textures(textures)
    }

    /// Handle input events from the window manager.
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        let _ = self.egui_state.on_event(&self.egui_ctx, event);
    }

    /// Resize egui.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.physical_width = width;
            self.screen_descriptor.physical_height = height;
        }
    }

    /// Update scaling factor.
    pub fn scale_factor(&mut self, scale_factor: f32) {
        self.screen_descriptor.scale_factor = scale_factor;
        self.egui_ctx.set_pixels_per_point(scale_factor);
    }

    /// The memory of EGUI with regard to windows and state.
    pub fn memory(&self) -> Memory {
        self.egui_ctx.memory(|mem| mem.clone())
    }
}

#[derive(Serialize, Deserialize)]
pub struct AppUiState {
    pub debug_ui: debug::UiState,
    pub egui: Memory,
}

/// Example application state. A real application will need a lot more state than this.
pub struct Gui {
    /// Only show the egui window when true.
    window_open: bool,

    pub debug_view: DebugViewManager,
}

impl Gui {
    /// Create a `Gui`.
    fn new(ui_state: Option<debug::UiState>) -> Self {
        Self {
            window_open: true,
            debug_view: DebugViewManager::new(ui_state),
        }
    }

    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context, state: &mut crate::State) {
        // let now = Instant::now();
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Emulation", |ui| {
                    if ui.checkbox(&mut state.paused, "Pause (K)").clicked() {
                        state.pause(state.paused);
                        ui.close_menu()
                    }

                    if ui.button("Reset").clicked() {
                        if let Some(emu) = state.current_emu.as_ref() {
                            emu.request_sender.send(EmulatorMessage::Reset).unwrap();
                        }
                        ui.close_menu()
                    }
                });

                self.debug_view.draw_menu_button(ui);
            });
        });

        let requests = self.debug_view.draw(ctx);

        if let Some(emu) = state.current_emu.as_ref() {
            for request in requests {
                emu.request_sender.send(EmulatorMessage::Debug(request)).unwrap();
            }
        }

        // println!("Egui Draw: {:?}", now.elapsed());
    }
}
