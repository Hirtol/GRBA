use crate::rendering::{Renderer, RendererOptions};
use crate::runner::{EmulatorRunner, RunnerHandle};
use crate::utils::BoolUtils;
use crate::RunningState::FastForward;
use egui_wgpu_backend::wgpu::{PresentMode, TextureFormat};
use grba_core::cartridge::header::CartridgeHeader;
use grba_core::cartridge::{Cartridge, CARTRIDGE_SRAM_START};
use log::LevelFilter;
use pixels::wgpu::{Backends, PowerPreference, RequestAdapterOptions};
use pixels::{wgpu, Pixels, PixelsBuilder, SurfaceTexture};
use std::fs::read;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

mod rendering;
mod runner;
mod utils;

fn main() {
    let cfg = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("grba_front")
        .build();

    simplelog::SimpleLogger::init(LevelFilter::Trace, cfg).unwrap();

    let event_loop = EventLoop::new();
    let mut input = winit_input_helper::WinitInputHelper::new();

    let mut renderer = Renderer::new(&event_loop, RendererOptions::default()).unwrap();
    let mut state = State::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            // Update renderer state and request new frame.
            renderer.after_window_update(&input);
        }

        match event {
            Event::WindowEvent { event, window_id } => {
                // Update egui inputs
                renderer.framework.handle_event(&event);

                if window_id != renderer.primary_window_id() {
                    return;
                }

                match event {
                    WindowEvent::DroppedFile(path) => {
                        log::debug!("Dropped file: {:?}", path);
                        let rom = handle_file_drop(path);

                        if let Some(rom) = rom {
                            state.load_cartridge(rom);
                        }
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        handle_key(input, &mut state, &mut renderer);
                    }
                    _ => {}
                };
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                let render_result = renderer.render_pixels(&[255; grba_core::FRAMEBUFFER_SIZE]);

                // Basic error handling
                if render_result.is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunningState {
    /// Run the emulator with a forced 60 fps frame limit, will cause audio desyncs once in a while
    FrameLimited,
    /// Run the emulator based on the audio playback rate, smooth audio playback, but will cause a skipped frame once in a while
    AudioLimited,
    /// Fast forward the emulator based on the provided multiplier.
    /// Providing `4` for example will cause the emulator to run 4 times faster than normal.
    FastForward(u8),
    /// Run the emulator as fast as possible.
    Unbounded,
}

struct State {
    /// The current emulation that is running
    pub(crate) current_emu: Option<RunnerHandle>,
    /// The title of the emulation that is running
    pub(crate) current_header: Option<CartridgeHeader>,
    /// How to run the emulator
    pub(crate) run_state: RunningState,
    /// Whether the emulator is paused
    pub(crate) paused: bool,
}

impl State {
    pub fn run_frame_limited(&mut self) {
        self.run_state = RunningState::FrameLimited;
    }

    pub fn run_audio_limited(&mut self) {
        self.run_state = RunningState::AudioLimited;
    }

    pub fn run_fast_forward(&mut self, multiplier: u8) {
        self.run_state = RunningState::FastForward(multiplier);
    }

    pub fn run_unbounded(&mut self) {
        self.run_state = RunningState::Unbounded;
    }

    pub fn run_default(&mut self) {
        self.run_state = RunningState::FrameLimited;
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            current_emu: None,
            current_header: None,
            run_state: RunningState::FrameLimited,
            paused: false,
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.current_header = Some(cartridge.header().clone());

        let runner = EmulatorRunner::new(cartridge);
        self.current_emu = Some(runner.run());
    }
}

fn handle_file_drop(path: PathBuf) -> Option<Cartridge> {
    let extension = path.extension()?.to_str()?;

    if extension == "gba" {
        let contents = std::fs::read(&path).ok()?;
        let parent_dir = path.parent()?;
        let file_name = path.file_name()?.to_string_lossy();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(parent_dir.join(format!("{}.bin", file_name)))
            .unwrap();
        file.set_len(grba_core::cartridge::CARTRIDGE_RAM_SIZE as u64).unwrap();

        let mut mm = memmap2::MmapOptions::new();
        let map = unsafe { mm.populate().map_mut(&file).ok()? };

        let cart = Cartridge::new(contents, Box::new(map));

        Some(cart)
    } else {
        None
    }
}

fn handle_key(input: KeyboardInput, state: &mut State, renderer: &mut Renderer) {
    // Handle emulator input.
    if let Some(emu) = &state.current_emu {
        emu.handle_input(input);
    }

    let key = if let Some(key) = input.virtual_keycode {
        key
    } else {
        return;
    };

    match key {
        VirtualKeyCode::U if input.state == ElementState::Released => {
            if state.run_state == RunningState::Unbounded {
                state.run_default();
            } else {
                state.run_unbounded();
            }
        }
        VirtualKeyCode::LShift => {
            if input.state == ElementState::Released {
                state.run_default();
            } else {
                state.run_frame_limited();
            }
        }
        VirtualKeyCode::K if input.state == ElementState::Released => {
            state.paused.toggle();
        }
        VirtualKeyCode::F11 if input.state == ElementState::Released => renderer.toggle_fullscreen(),
        _ => {}
    }
}
