use crossbeam::channel::Sender;
use egui::{ClippedMesh, Context, TexturesDelta};
use egui_wgpu_backend::{BackendError, RenderPass, ScreenDescriptor};
use pixels::{wgpu, PixelsContext};
use std::time::Instant;
use winit::window::Window;

use crate::runner::messages::EmulatorMessage;
pub use debug::messages::{DebugMessageResponse, DebugMessageUi};
pub use debug::DebugViewManager;

mod debug;

/// Manages all state required for rendering egui over `Pixels`.
pub struct Framework {
    // State for egui.
    egui_ctx: Context,
    // Egui Winit helper
    egui_state: egui_winit::State,
    // Egui WebGPU backend
    rpass: RenderPass,
    screen_descriptor: ScreenDescriptor,
    paint_jobs: Vec<ClippedMesh>,
    textures: TexturesDelta,

    // State for the GUI
    pub gui: Gui,
}

impl Framework {
    /// Create egui.
    pub(crate) fn new(width: u32, height: u32, scale_factor: f32, pixels: &pixels::Pixels) -> Self {
        let egui_ctx = Context::default();
        let egui_state = egui_winit::State::from_pixels_per_point(2048, scale_factor);
        let screen_descriptor = ScreenDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor,
        };
        let rpass = RenderPass::new(pixels.device(), pixels.render_texture_format(), 1);
        let gui = Gui::new();

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

    /// Handle input events from the window manager.
    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui_state.on_event(&self.egui_ctx, event);
    }

    /// Resize egui.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.physical_width = width;
            self.screen_descriptor.physical_height = height;
        }
    }

    /// Update scaling factor.
    pub(crate) fn scale_factor(&mut self, scale_factor: f32) {
        self.screen_descriptor.scale_factor = scale_factor;
        self.egui_ctx.set_pixels_per_point(scale_factor);
    }

    /// Prepare egui.
    pub(crate) fn prepare(&mut self, window: &Window, request_sender: Option<&mut Sender<EmulatorMessage>>) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx, request_sender);
        });

        self.textures.append(full_output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(full_output.shapes);
    }

    /// Render egui.
    pub(crate) fn render(
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
}

/// Example application state. A real application will need a lot more state than this.
pub struct Gui {
    /// Only show the egui window when true.
    window_open: bool,

    pub debug_view: DebugViewManager,
}

impl Gui {
    /// Create a `Gui`.
    fn new() -> Self {
        Self {
            window_open: true,
            debug_view: DebugViewManager::new(),
        }
    }

    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context, request_sender: Option<&mut Sender<EmulatorMessage>>) {
        // let now = Instant::now();
        let requests = self.debug_view.draw(ctx);

        if let Some(sender) = request_sender {
            for request in requests {
                sender.send(EmulatorMessage::Debug(request)).unwrap();
            }
        }

        egui::Window::new("Hello, egui!")
            .open(&mut self.window_open)
            .show(ctx, |ui| {
                ui.label("This example demonstrates using egui with pixels.");
                ui.label("Made with 💖 in San Francisco!");

                ui.separator();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about egui at");
                    ui.hyperlink("https://docs.rs/egui");
                });
            });

        // println!("Egui Draw: {:?}", now.elapsed());
    }
}
