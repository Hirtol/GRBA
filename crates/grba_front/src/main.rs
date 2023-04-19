use std::path::PathBuf;
use std::time::{Duration, Instant};

use log::LevelFilter;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use grba_core::emulator::cartridge::header::CartridgeHeader;
use grba_core::emulator::cartridge::Cartridge;

use crate::gui::EguiFramework;
use crate::rendering::{Renderer, RendererOptions};
use crate::runner::messages::EmulatorResponse;
use crate::runner::{EmulatorRunner, RunnerHandle};
use crate::utils::MainArgs;

pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 720;

mod config;
mod debug;
pub mod gui;
mod rendering;
mod runner;
mod utils;

fn main() {
    let cfg = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("grba_front")
        .build();

    simplelog::SimpleLogger::init(LevelFilter::Trace, cfg).unwrap();

    #[cfg(feature = "bin-logging")]
    debug::setup_emulator_logger("./emu.logbin").expect("Failed to setup bin logger");

    let cli_options = utils::parse_main_args().expect("Failed to parse arguments");
    let application = Application::new(cli_options).expect("Failed to create application");

    let _ = application.run();
}

struct Application {
    state: State,
    gui: EguiFramework,
    renderer: Renderer,

    input: winit_input_helper::WinitInputHelper,

    event_loop: EventLoop<()>,
    wait_to: Instant,
    start: Instant,
}

impl Application {
    // FRAME_DURATION == Duration::from_secs_f32(1.0 / grba_core::REFRESH_RATE);
    // sadly, no f32 in const context :(
    const FRAME_DURATION: Duration = Duration::from_nanos(16742706);

    pub fn new(cli_options: MainArgs) -> anyhow::Result<Application> {
        let gui_state = config::deserialise_state_and_config();
        let event_loop = EventLoop::new();
        let input = winit_input_helper::WinitInputHelper::new();
        let renderer = Renderer::new(&event_loop, RendererOptions::default())?;
        let gui = EguiFramework::new(
            crate::WIDTH,
            crate::HEIGHT,
            renderer.scale_factor(),
            &renderer.pixels,
            &event_loop,
            gui_state,
        );

        Ok(Application {
            state: State::new(cli_options),
            gui,
            renderer,
            input,
            event_loop,
            wait_to: Instant::now(),
            start: Instant::now(),
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.event_loop.run(move |event, _window, control_flow| {
            // Handle input events
            if self.input.update(&event) {
                // Close events
                if self.input.key_pressed(VirtualKeyCode::Escape) || self.input.quit() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                // Update renderer state and request new frame.
                self.renderer.after_window_update(&self.input, &mut self.gui);
            }

            match event {
                Event::WindowEvent { event, window_id } => {
                    // Update egui inputs
                    self.gui.handle_event(&event);

                    if window_id != self.renderer.primary_window_id() {
                        return;
                    }

                    match event {
                        WindowEvent::DroppedFile(path) => {
                            log::debug!("Dropped file: {:?}", path);
                            let rom = load_gba_cartridge(path);

                            if let Some(rom) = rom {
                                self.state.load_cartridge(rom);
                            }
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            handle_key(input, &mut self.state, &mut self.renderer);
                        }
                        _ => {}
                    };
                }
                // Draw the current frame
                Event::RedrawRequested(_) => {
                    if self.state.current_emu.is_none() {
                        // No emu, don't draw excessively.
                        *control_flow = ControlFlow::WaitUntil(Instant::now() + Self::FRAME_DURATION);

                        let now = Instant::now();

                        if now <= self.wait_to {
                            *control_flow = ControlFlow::WaitUntil(self.wait_to);

                            let render_result = self.renderer.render_pixels(
                                &[0; grba_core::FRAMEBUFFER_SIZE * 4],
                                &mut self.gui,
                                &mut self.state,
                            );

                            // Basic error handling
                            if render_result.is_err() {
                                *control_flow = ControlFlow::Exit;
                            }
                        } else {
                            self.wait_to += Self::FRAME_DURATION;
                        }

                        return;
                    } else {
                        // We have an emulator, so run as fast as we can.
                        *control_flow = ControlFlow::Poll
                    }

                    let error = if self.state.paused {
                        // If paused just wait
                        Self::handle_paused(&mut self.state, &mut self.renderer, &mut self.gui, control_flow)
                    } else {
                        Self::handle_draw(
                            &mut self.state,
                            &mut self.renderer,
                            &mut self.gui,
                            &mut self.wait_to,
                            control_flow,
                        )
                    };

                    if let Err(e) = error {
                        *control_flow = ControlFlow::Exit;
                        log::error!("Failed to render {:#}", e);
                    }
                }
                Event::LoopDestroyed => {
                    config::save_state_and_config(&self.gui).expect("Failed to save state & config");
                }
                _ => (),
            }
        });
    }

    fn handle_paused(
        state: &mut State,
        renderer: &mut Renderer,
        gui: &mut EguiFramework,
        control_flow: &mut ControlFlow,
    ) -> anyhow::Result<()> {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Self::FRAME_DURATION);

        let frame = match &mut state.current_emu {
            Some(emu) => {
                // Handle emulator responses to our messages
                Self::handle_debug_messages(gui, control_flow, emu);

                // Try receive a frame to clear up the emulator in case it's waiting for a new frame to come in.
                // Converting to vec is dreadful, but pulling the emulator out of state is worse atm.
                emu.frame_receiver.try_recv_or_recent().as_bytes().to_vec()
            }
            None => vec![0; grba_core::FRAMEBUFFER_SIZE * 4],
        };

        renderer.render_pixels(&frame, gui, state)?;

        Ok(())
    }

    fn handle_draw(
        state: &mut State,
        renderer: &mut Renderer,
        gui: &mut EguiFramework,
        wait_to: &mut Instant,
        control_flow: &mut ControlFlow,
    ) -> anyhow::Result<()> {
        // Determine if we need to wait.
        match state.run_state {
            RunningState::FrameLimited | RunningState::FastForward(_) => {
                let now = Instant::now();

                if now <= *wait_to {
                    *control_flow = ControlFlow::WaitUntil(*wait_to);
                    return Ok(());
                } else {
                    *wait_to += Self::FRAME_DURATION;
                }
            }
            RunningState::AudioLimited => {
                todo!()
            }
            RunningState::Unbounded => {
                *wait_to = Instant::now();
            }
        }

        // Need to render a frame.
        let frames_to_render = match state.run_state {
            RunningState::FastForward(frames) => frames,
            _ => 1,
        };

        for _ in 0..frames_to_render {
            let frame = {
                let emu = state.current_emu.as_mut().unwrap();
                // Handle emulator responses to our messages
                Self::handle_debug_messages(gui, control_flow, emu);

                emu.frame_receiver.recv()?.as_bytes().to_vec()
            };

            // Render result and send debug requests
            renderer.render_pixels(&frame, gui, state)?;
        }

        Ok(())
    }

    fn handle_debug_messages(gui: &mut EguiFramework, _control_flow: &mut ControlFlow, emu: &mut RunnerHandle) {
        while let Ok(response) = emu.response_receiver.try_recv() {
            match response {
                EmulatorResponse::Debug(msg) => gui.gui.debug_view.handle_response_message(msg),
            }
        }
    }
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

pub struct State {
    /// The current emulation that is running
    pub current_emu: Option<RunnerHandle>,
    /// The title of the emulation that is running
    pub current_header: Option<CartridgeHeader>,
    /// How to run the emulator
    pub run_state: RunningState,
    /// Whether the emulator is paused
    pub paused: bool,
    /// The location of the BIOS file.
    pub bios_location: PathBuf,
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
    pub fn new(cli_options: MainArgs) -> Self {
        let mut result = Self {
            current_emu: None,
            current_header: None,
            run_state: RunningState::FrameLimited,
            paused: false,
            bios_location: cli_options.bios,
        };

        // Set the initial state according to our CLI parameters
        if let Some(initial_rom) = cli_options.execute_path {
            let cartridge = load_gba_cartridge(initial_rom).expect("Initial ROM was an invalid GBA cartridge");
            result.load_cartridge(cartridge);
            result.pause(cli_options.start_paused)
        }

        result
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.current_header = Some(cartridge.header().clone());
        let bios = std::fs::read(&self.bios_location).unwrap();

        let runner = EmulatorRunner::new(cartridge, Some(bios));
        self.current_emu = Some(runner.run(self.paused));
    }

    pub fn pause(&mut self, pause: bool) {
        log::debug!("Pausing: {}", pause);
        self.paused = pause;

        // Send a message to the emulator thread to pause/unpause
        if let Some(emu) = &self.current_emu {
            if pause {
                let _ = emu.pause();
            } else {
                let _ = emu.unpause();
            }
        }
    }
}

fn load_gba_cartridge(path: PathBuf) -> Option<Cartridge> {
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
        file.set_len(grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE as u64)
            .unwrap();

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
            log::debug!("Run State: {:?}", state.run_state);
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
                state.run_fast_forward(4);
            }
        }
        VirtualKeyCode::K if input.state == ElementState::Released => {
            state.pause(!state.paused);
        }
        VirtualKeyCode::F11 if input.state == ElementState::Released => renderer.toggle_fullscreen(),
        _ => {}
    }
}
