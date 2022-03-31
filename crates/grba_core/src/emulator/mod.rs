use crate::emulator::bus::BiosData;
use crate::emulator::ppu::RGBA;
use crate::scheduler::EventTag;
use crate::{InputKeys, CLOCKS_PER_FRAME, FRAMEBUFFER_SIZE};
use bus::Bus;
use cartridge::Cartridge;
use cpu::CPU;

mod bus;
pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod debugging;
pub mod ppu;

/// Refers to an *absolute* memory address.
/// Therefore any component which takes this as an incoming type *must* pre-process the value to turn it into an address
/// relative to itself.
pub type MemoryAddress = u32;

#[derive(Debug)]
pub struct EmuOptions {
    /// Whether to skip the bios.
    /// This is automatically enabled if no BIOS is provided.
    pub skip_bios: bool,
    /// BIOS to use.
    /// If none is provided then the bios region of memory will be zeroed out, and `skip_bios` will be forcefully enabled.
    pub bios: Option<Vec<u8>>,
    /// `true` if the emulator should run in debug mode.
    /// This will enable breakpoints.
    pub debugging: bool,
}

impl Default for EmuOptions {
    fn default() -> Self {
        EmuOptions {
            skip_bios: true,
            bios: None,
            debugging: false,
        }
    }
}

/// The main emulator struct
pub struct GBAEmulator {
    pub(crate) cpu: CPU,
    pub(crate) bus: Bus,
    options: EmuOptions,
}

impl GBAEmulator {
    pub fn new(rom: Cartridge, mut options: EmuOptions) -> Self {
        let has_bios = options.bios.is_some();
        let mut mmu = Bus::new(rom, vec_to_bios_data(options.bios.take()));

        GBAEmulator {
            cpu: CPU::new(options.skip_bios || !has_bios, &mut mmu),
            bus: mmu,
            options,
        }
    }

    /// Run the emulator until it has reached Vblank
    #[profiling::function]
    pub fn run_to_vblank(&mut self) {
        // We split on the debugging option here to incur as little runtime overhead as possible.
        // If we need more thorough debugging abilities in the future we'll probably need to look at generics instead.
        if self.options.debugging {
            while !self.step_instruction_debug() {}
        } else {
            while !self.step_instruction() {}
        }
        profiling::finish_frame!();
    }

    pub fn step_instruction(&mut self) -> bool {
        self.cpu.step_instruction(&mut self.bus);

        // Very basic cycle counting to get things going. In the future ought to count cycles properly.
        //TODO: Instruction timing
        self.bus.scheduler.add_time(2);

        while let Some(event) = self.bus.scheduler.pop_current() {
            match event.tag {
                EventTag::Exit => {
                    panic!("Exit shouldn't ever be triggered!");
                }
                EventTag::VBlank => {
                    self.bus.ppu.vblank(&mut self.bus.scheduler, &mut self.bus.interrupts);
                    return true;
                }
                EventTag::HBlank => {
                    self.bus
                        .ppu
                        .hblank_start(&mut self.bus.scheduler, &mut self.bus.interrupts);
                }
                EventTag::HBlankEnd => {
                    self.bus
                        .ppu
                        .hblank_end(&mut self.bus.scheduler, &mut self.bus.interrupts);
                }
                EventTag::PollInterrupt => {
                    self.cpu.poll_interrupts(&mut self.bus);
                }
            }
        }

        false
    }

    pub fn step_instruction_debug(&mut self) -> bool {
        self.step_instruction()
    }

    pub fn key_down(&mut self, key: InputKeys) {
        self.bus
            .keypad
            .button_changed(key, true, &mut self.bus.scheduler, &mut self.bus.interrupts);
    }

    pub fn key_up(&mut self, key: InputKeys) {
        self.bus
            .keypad
            .button_changed(key, false, &mut self.bus.scheduler, &mut self.bus.interrupts);
    }

    pub fn frame_buffer(&mut self) -> &mut Box<[RGBA; FRAMEBUFFER_SIZE]> {
        self.bus.ppu.frame_buffer()
    }
}

fn vec_to_bios_data(data: Option<Vec<u8>>) -> Box<BiosData> {
    let data = data.unwrap_or_else(|| vec![0; std::mem::size_of::<BiosData>()]);
    Box::try_from(data.into_boxed_slice()).unwrap()
}
