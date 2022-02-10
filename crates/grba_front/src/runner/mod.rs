use crate::runner::messages::{EmulatorMessage, EmulatorResponse};
use crossbeam::channel::{bounded, unbounded, Receiver, Sender};

use grba_core::emulator::cartridge::Cartridge;
use grba_core::emulator::EmuOptions;
use grba_core::emulator::GBAEmulator;
use grba_core::InputKeys;
use std::thread::JoinHandle;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub mod messages;

pub struct EmulatorRunner {
    rom: Cartridge,
}

impl EmulatorRunner {
    pub fn new(rom: Cartridge) -> Self {
        Self { rom }
    }

    pub fn run(self) -> RunnerHandle {
        let (frame_sender, frame_receiver) = bounded(1);
        let (request_sender, request_receiver) = unbounded::<EmulatorMessage>();
        let (response_sender, response_receiver) = unbounded::<EmulatorResponse>();

        let emu_thread = std::thread::spawn(move || {
            profiling::register_thread!("Emulator Thread");
            let mut emulator = create_emulator(self.rom, EmuOptions::default());
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
    pub frame_receiver: Receiver<Vec<u8>>,
    pub request_sender: Sender<EmulatorMessage>,
    pub response_receiver: Receiver<EmulatorResponse>,
}

impl RunnerHandle {
    /// Inform the emulator of a keypress event.
    pub fn handle_input(&self, input: KeyboardInput) {
        let key = if let Some(key) = keyboard_to_input(input) { key } else { return };

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

    /// Stops the current emulator thread and blocks until it has completed.
    pub fn stop(self) {
        let _ = self.request_sender.send(EmulatorMessage::ExitRequest);
        // Since the emulation thread may be blocking trying to send a frame.
        let _ = self.frame_receiver.try_recv();

        self.current_thread.join().expect("Failed to join emulator thread");
    }
}

fn run_emulator(
    emu: &mut GBAEmulator,
    frame_sender: Sender<Vec<u8>>,
    _response_sender: Sender<EmulatorResponse>,
    request_receiver: Receiver<EmulatorMessage>,
) {
    loop {
        profiling::scope!("Emulator Loop");
        emu.run_to_vblank();

        if let Err(e) = frame_sender.send(emu.frame_buffer()) {
            log::error!("Failed to transfer framebuffer due to: {:?}", e);
            break;
        }

        while let Ok(msg) = request_receiver.try_recv() {
            match msg {
                EmulatorMessage::ExitRequest => break,
                EmulatorMessage::KeyDown(key) => emu.key_down(key),
                EmulatorMessage::KeyUp(key) => emu.key_up(key),
                EmulatorMessage::Debug(msg) => {
                    println!("{:?}", msg);
                }
            }
        }
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
