use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::CPU;
use crate::{InputKeys, CLOCKS_PER_FRAME};

/// Refers to an *absolute* memory address.
/// Therefore any component which takes this as an incoming type *must* pre-process the value to turn it into an address
/// relative to itself.
pub(crate) type MemoryAddress = u32;

#[derive(Debug)]
pub struct EmuOptions {
    pub skip_bios: bool,
}

impl Default for EmuOptions {
    fn default() -> Self {
        EmuOptions { skip_bios: true }
    }
}

/// The main emulator struct
pub struct GBAEmulator {
    pub(crate) cpu: CPU,
    pub(crate) mmu: Bus,
}

impl GBAEmulator {
    pub fn new(rom: Cartridge, options: EmuOptions) -> Self {
        let mut mmu = Bus::new(rom);

        GBAEmulator {
            cpu: CPU::new(options.skip_bios, &mut mmu),
            mmu,
        }
    }

    /// Run the emulator until it has reached Vblank
    #[profiling::function]
    pub fn run_to_vblank(&mut self) {
        while !self.emulate_cycle() {}
        profiling::finish_frame!();
    }

    pub fn emulate_cycle(&mut self) -> bool {
        self.cpu.step_instruction(&mut self.mmu);
        // Temporary measure to get some frames.
        (self.mmu.scheduler.current_time.0 % CLOCKS_PER_FRAME as u64) == 0
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
