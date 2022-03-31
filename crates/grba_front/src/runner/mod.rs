use crate::runner::messages::{EmulatorMessage, EmulatorResponse};
use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::rendering::gui::DebugViewManager;
use crate::runner::frame_exchanger::{ExchangerReceiver, ExchangerSender};
use grba_core::emulator::cartridge::Cartridge;
use grba_core::emulator::debug::DebugEmulator;
use grba_core::emulator::frame::RgbaFrame;
use grba_core::emulator::ppu::RGBA;
use grba_core::emulator::EmuOptions;
use grba_core::emulator::GBAEmulator;
use grba_core::InputKeys;
use std::thread::JoinHandle;
use std::time::Duration;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub mod frame_exchanger;
pub mod messages;

pub struct EmulatorRunner {
    rom: Cartridge,
    bios: Option<Vec<u8>>,
}

impl EmulatorRunner {
    pub fn new(rom: Cartridge, bios: Option<Vec<u8>>) -> Self {
        Self { rom, bios }
    }

    pub fn run(self) -> RunnerHandle {
        let (request_sender, request_receiver) = unbounded::<EmulatorMessage>();
        let (response_sender, response_receiver) = unbounded::<EmulatorResponse>();
        let (frame_sender, frame_receiver) = frame_exchanger::exchangers(RgbaFrame::default());

        let emu_thread = std::thread::spawn(move || {
            profiling::register_thread!("Emulator Thread");

            let emu_options = EmuOptions {
                bios: self.bios,
                ..Default::default()
            };

            let mut emulator = create_emulator(self.rom, emu_options);
            run_emulator(&mut emulator, frame_sender, response_sender, request_receiver);
        });

        RunnerHandle {
            current_thread: emu_thread,
            frame_receiver,
            request_sender,
            response_receiver,
        }
    }
}

pub struct RunnerHandle {
    current_thread: JoinHandle<()>,
    pub frame_receiver: ExchangerReceiver<RgbaFrame>,
    pub request_sender: Sender<EmulatorMessage>,
    pub response_receiver: Receiver<EmulatorResponse>,
}

impl RunnerHandle {
    /// Inform the emulator of a keypress event.
    pub fn handle_input(&self, input: KeyboardInput) {
        let key = if let Some(key) = keyboard_to_input(input) { key } else { return };
        println!("Sending input: {:?} - {:?}", key, input.state);
        if input.state == ElementState::Pressed {
            self.request_sender
                .send(EmulatorMessage::KeyDown(key))
                .expect("Failed to send key down message");
        } else {
            self.request_sender
                .send(EmulatorMessage::KeyUp(key))
                .expect("Failed to send key up message");
        }
    }

    /// Pause the emulator, but continue serving other requests.
    pub fn pause(&self) -> anyhow::Result<()> {
        self.request_sender.send(EmulatorMessage::Pause)?;

        Ok(())
    }

    /// Unpause the emulator.
    pub fn unpause(&self) -> anyhow::Result<()> {
        self.request_sender.send(EmulatorMessage::Unpause)?;

        Ok(())
    }

    /// Stops the current emulator thread and blocks until it has completed.
    pub fn stop(mut self) {
        let _ = self.request_sender.send(EmulatorMessage::ExitRequest);
        // Since the emulation thread may be blocking trying to send a frame.
        let _ = self.frame_receiver.try_recv();

        self.current_thread.join().expect("Failed to join emulator thread");
    }
}

fn run_emulator(
    emu: &mut GBAEmulator,
    frame_sender: ExchangerSender<RgbaFrame>,
    response_sender: Sender<EmulatorResponse>,
    request_receiver: Receiver<EmulatorMessage>,
) {
    'mainloop: loop {
        profiling::scope!("Emulator Loop");

        emu.run_to_vblank();

        if let Err(e) = frame_sender.send(emu.frame_buffer()) {
            log::error!("Failed to transfer framebuffer due to: {:#}", e);
            break;
        }

        while let Ok(msg) = request_receiver.try_recv() {
            match msg {
                EmulatorMessage::ExitRequest => break 'mainloop,
                EmulatorMessage::KeyDown(key) => emu.key_down(key),
                EmulatorMessage::KeyUp(key) => emu.key_up(key),
                EmulatorMessage::Debug(msg) => {
                    let mut emu = DebugEmulator(emu);
                    let response = DebugViewManager::handle_ui_request_message(&mut emu, msg);

                    response_sender
                        .send(EmulatorResponse::Debug(response))
                        .expect("Failed to send response");
                }
                EmulatorMessage::Pause => {
                    if pause_loop(emu, &response_sender, &request_receiver) {
                        break 'mainloop;
                    }
                }
                EmulatorMessage::Unpause => {
                    log::info!("Tried to unpause when not paused");
                }
            }
        }
    }
}

/// Enter into the pause loop for the emulator.
///
/// # Returns
/// * `true` - If the emulator receives an exit command while it is paused.
/// * `false` - When the emulator receives the unpause command.
#[inline(never)]
fn pause_loop(
    emu: &mut GBAEmulator,
    response_sender: &Sender<EmulatorResponse>,
    request_receiver: &Receiver<EmulatorMessage>,
) -> bool {
    'pause_loop: loop {
        while let Ok(msg) = request_receiver.try_recv() {
            match msg {
                EmulatorMessage::ExitRequest => break 'pause_loop true,
                EmulatorMessage::KeyDown(key) => emu.key_down(key),
                EmulatorMessage::KeyUp(key) => emu.key_up(key),
                EmulatorMessage::Debug(msg) => {
                    let mut emu = DebugEmulator(emu);
                    let response = DebugViewManager::handle_ui_request_message(&mut emu, msg);

                    response_sender
                        .send(EmulatorResponse::Debug(response))
                        .expect("Failed to send response");
                }
                EmulatorMessage::Pause => log::info!("Tried to pause when already paused"),
                EmulatorMessage::Unpause => break 'pause_loop false,
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}

fn create_emulator(rom: Cartridge, options: EmuOptions) -> GBAEmulator {
    log::info!("Created emulator for ROM: {:#?}", rom.header());
    GBAEmulator::new(rom, options)
}

fn keyboard_to_input(input: KeyboardInput) -> Option<InputKeys> {
    match input.virtual_keycode? {
        VirtualKeyCode::Up => Some(InputKeys::Up),
        VirtualKeyCode::Down => Some(InputKeys::Down),
        VirtualKeyCode::Left => Some(InputKeys::Left),
        VirtualKeyCode::Right => Some(InputKeys::Right),
        VirtualKeyCode::A => Some(InputKeys::A),
        VirtualKeyCode::B => Some(InputKeys::B),
        VirtualKeyCode::S => Some(InputKeys::Select),
        VirtualKeyCode::T => Some(InputKeys::Start),
        VirtualKeyCode::Q => Some(InputKeys::ShoulderLeft),
        VirtualKeyCode::E => Some(InputKeys::ShoulderRight),
        _ => None,
    }
}
