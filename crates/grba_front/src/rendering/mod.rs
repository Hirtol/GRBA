use crate::gui::EguiFramework;
use crate::rendering::framerate::FrameRate;
use crate::runner::messages::EmulatorMessage;
use crate::{gui, State};
use anyhow::Context;
use crossbeam::channel::Sender;
use pixels::Pixels;
use std::time::Instant;
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Window, WindowId};

mod framerate;

pub const SCALE_FACTOR_MULTIPLIER: f32 = 1.2;

#[derive(Debug, Clone)]
pub struct RendererOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            title: "GRBA".to_string(),
            width: crate::WIDTH,
            height: crate::HEIGHT,
        }
    }
}

pub struct Renderer {
    pub pixels: Pixels,
    primary_window: Window,
    framerate: framerate::FrameRate,
    last_title_update: Instant,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, options: RendererOptions) -> anyhow::Result<Self> {
        let window = {
            let size = winit::dpi::LogicalSize::new(options.width as f64, options.height as f64);
            winit::window::WindowBuilder::new()
                .with_title(options.title)
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)?
        };

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture = pixels::SurfaceTexture::new(window_size.width, window_size.height, &window);

            let pixels =
                pixels::PixelsBuilder::new(grba_core::DISPLAY_WIDTH, grba_core::DISPLAY_HEIGHT, surface_texture)
                    .request_adapter_options(wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        force_fallback_adapter: false,
                        compatible_surface: None,
                    })
                    .present_mode(wgpu::PresentMode::Immediate)
                    .build()?;

            pixels
        };

        Ok(Self {
            pixels,
            primary_window: window,
            framerate: FrameRate::new(),
            last_title_update: Instant::now(),
        })
    }

    /// Renders the main window's contents (The framebuffer).
    pub fn render_pixels(
        &mut self,
        framebuffer: &[u8],
        gui: &mut EguiFramework,
        state: &mut State,
    ) -> anyhow::Result<()> {
        let frame = self.pixels.get_frame_mut();

        frame.copy_from_slice(framebuffer);

        gui.prepare(&self.primary_window, state);

        // Render everything together
        let result = self
            .pixels
            .render_with(|encoder, render_target, context| {
                // Render the world texture
                context.scaling_renderer.render(encoder, render_target);

                // Render egui
                gui.render(encoder, render_target, context)?;

                Ok(())
            })
            .context("Failed to render pixels");

        self.framerate.frame_finished();

        result
    }

    /// For when using a second window as the Egui interface.
    pub fn render_immediate_mode(&mut self) {
        todo!()
    }

    /// To be called after `input.update(event)` returns `true`
    ///
    /// Will update the scale factor, as well as handle window resize events for both `egui` and `pixels`.
    /// Lastly, it will request a redraw.
    pub fn after_window_update(&mut self, input: &winit_input_helper::WinitInputHelper, gui: &mut EguiFramework) {
        // Update the scale factor
        if let Some(scale_factor) = input.scale_factor() {
            gui.scale_factor(scale_factor as f32 * SCALE_FACTOR_MULTIPLIER);
        }

        // Resize the window
        if let Some(size) = input.window_resized() {
            self.pixels.resize_surface(size.width, size.height);
            gui.resize(size.width, size.height);
        }

        // Update window title
        if self.last_title_update.elapsed().as_secs() >= 1 {
            let fps = self.framerate.fps();
            self.primary_window
                .set_title(&format!("GRBA - [{:.1} FPS | {:.0}%]", fps, fps / 60.0 * 100.0));
            self.last_title_update = Instant::now();
        }

        // Update internal state and request a redraw
        self.request_redraw();
    }

    pub fn scale_factor(&self) -> f32 {
        self.primary_window.scale_factor() as f32 * SCALE_FACTOR_MULTIPLIER
    }

    pub fn fps(&self) -> f32 {
        self.framerate.fps()
    }

    pub fn request_redraw(&self) {
        self.primary_window.request_redraw();
    }

    pub fn primary_window_id(&self) -> WindowId {
        self.primary_window.id()
    }

    pub fn toggle_fullscreen(&self) {
        match self.primary_window.fullscreen() {
            None => {
                self.primary_window
                    .set_fullscreen(Some(Fullscreen::Borderless(self.primary_window.current_monitor())));
            }
            Some(_) => self.primary_window.set_fullscreen(None),
        }
    }
}
