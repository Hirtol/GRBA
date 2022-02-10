use bus::Bus;
use cartridge::Cartridge;
use cpu::CPU;
use crate::{CLOCKS_PER_FRAME, InputKeys};

mod bus;
pub mod cartridge;
pub mod cpu;
pub mod ppu;
pub mod debugging;

/// Refers to an *absolute* memory address.
/// Therefore any component which takes this as an incoming type *must* pre-process the value to turn it into an address
/// relative to itself.
pub(crate) type MemoryAddress = u32;

#[derive(Debug)]
pub struct EmuOptions {
    pub skip_bios: bool,
    /// `true` if the emulator should run in debug mode.
    /// This will enable breakpoints.
    pub debugging: bool
}

impl Default for EmuOptions {
    fn default() -> Self {
        EmuOptions { skip_bios: true, debugging: false }
    }
}

/// The main emulator struct
pub struct GBAEmulator {
    pub(crate) cpu: CPU,
    pub(crate) mmu: Bus,
    options: EmuOptions,
}

impl GBAEmulator {
    pub fn new(rom: Cartridge, options: EmuOptions) -> Self {
        let mut mmu = Bus::new(rom);

        GBAEmulator {
            cpu: CPU::new(options.skip_bios, &mut mmu),
            mmu,
            options
        }
    }

    /// Run the emulator until it has reached Vblank
    #[profiling::function]
    pub fn run_to_vblank(&mut self) {
        // We split on the debugging option here to incur as little runtime overhead as possible.
        // If we need more thorough debugging abilities in the future we'll probably need to look at generics instead.
        if self.options.debugging {
            while !self.step_instruction_debug() {};
        } else {
            while !self.step_instruction() {};
        }
        profiling::finish_frame!();
    }

    pub fn step_instruction(&mut self) -> bool {
        self.cpu.step_instruction(&mut self.mmu);
        // Temporary measure to get some frames.
        (self.mmu.scheduler.current_time.0 % CLOCKS_PER_FRAME as u64) == 0
    }
    
    pub fn step_instruction_debug(&mut self) -> bool {
        self.step_instruction()
    }

    pub fn key_down(&self, key: InputKeys) {
        //TODO
    }

    pub fn key_up(&self, key: InputKeys) {
        //TODO
    }

    pub fn frame_buffer(&self) -> Vec<u8> {
        vec![180; crate::FRAMEBUFFER_SIZE]
    }

    pub fn frame_buffer_ref(&mut self) -> &mut Vec<u8> {
        todo!()
    }

    #[inline(always)]
    fn frame_data(&mut self) -> FrameData {
        FrameData { mmu: &mut self.mmu }
    }
}

#[repr(transparent)]
pub(crate) struct FrameData<'a> {
    pub mmu: &'a mut Bus,
}
