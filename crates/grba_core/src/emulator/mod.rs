use bus::Bus;
use cartridge::Cartridge;
use cpu::CPU;

use crate::emulator::bus::BiosData;
use crate::emulator::frame::RgbaFrame;
use crate::scheduler::{EmuTime, Event, EventTag};
use crate::InputKeys;

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod frame;
pub mod ppu;

/// Refers to an *absolute* memory address.
/// Therefore any component which takes this as an incoming type *must* pre-process the value to turn it into an address
/// relative to itself.
pub type MemoryAddress = u32;
pub type AlignedAddress = u32;

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

pub struct EmuDebugging {
    pub breakpoints: Vec<MemoryAddress>,
    pub break_at_cycle: Option<u64>,
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
    pub(crate) debug: EmuDebugging,
    pub options: EmuOptions,
}

impl GBAEmulator {
    pub fn new(rom: Cartridge, mut options: EmuOptions) -> Self {
        let has_bios = options.bios.is_some();
        let mut mmu = Bus::new(rom, vec_to_bios_data(options.bios.take()));

        GBAEmulator {
            cpu: CPU::new(options.skip_bios || !has_bios, &mut mmu),
            bus: mmu,
            options,
            debug: EmuDebugging {
                breakpoints: Vec::new(),
                break_at_cycle: None,
            },
        }
    }

    /// Run the emulator until it has reached Vblank
    #[profiling::function]
    pub fn run_to_vblank(&mut self) {
        while !self.step_instruction() {}
        profiling::finish_frame!();
    }

    /// Run the emulator until it has reached `Vblank`.
    ///
    /// # Returns
    ///
    /// `true` if the emulator hit a breakpoint, stopping execution early.
    pub fn run_to_vblank_debug(&mut self) -> bool {
        loop {
            let (vblank, breakpoint) = self.step_instruction_debug();

            if breakpoint {
                println!("Breakpoint hit!");
                return true;
            } else if vblank {
                return false;
            }
        }
    }

    /// Step the emulator for a single instruction.
    ///
    /// # Returns
    ///
    /// `true` if `Vblank` was reached, `false` otherwise.
    #[inline(always)]
    pub fn step_instruction(&mut self) -> bool {
        self.cpu.step_instruction(&mut self.bus);

        // Very basic cycle counting to get things going. In the future ought to count cycles properly.
        //TODO: Instruction timing
        self.bus.scheduler.add_time(2);

        while let Some(event) = self.bus.scheduler.pop_current() {
            if self.handle_scheduled_event(event) {
                return true;
            }
        }

        false
    }

    /// Handle a scheduled event.
    ///
    /// # Returns
    ///
    /// * `true` if VBLANK was hit for this event.
    /// * `false` in all other cases.
    #[inline(always)]
    fn handle_scheduled_event(&mut self, event: Event) -> bool {
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
            EventTag::Halt => {
                println!("Halting!");

                // If this returns `true` then we've hit Vblank and should exit for now.
                if self.handle_halt() {
                    return true;
                }
            }
            EventTag::Timer0Irq => {
                self.bus
                    .timers
                    .timer_overflowed(0, &mut self.bus.scheduler, event.timestamp, &mut self.bus.interrupts);
            }
            EventTag::Timer1Irq => {
                self.bus
                    .timers
                    .timer_overflowed(1, &mut self.bus.scheduler, event.timestamp, &mut self.bus.interrupts);
            }
            EventTag::Timer2Irq => {
                self.bus
                    .timers
                    .timer_overflowed(2, &mut self.bus.scheduler, event.timestamp, &mut self.bus.interrupts);
            }
            EventTag::Timer3Irq => {
                self.bus
                    .timers
                    .timer_overflowed(3, &mut self.bus.scheduler, event.timestamp, &mut self.bus.interrupts);
            }
        }

        false
    }

    /// Will handle the loop to handle the HALT state.
    ///
    /// # Returns
    ///
    /// * `true` if VBLANK is hit during the HALT state, a new HALT event will be scheduled as the next event to be executed.
    /// * `false` if the HALT state is exited.
    fn handle_halt(&mut self) -> bool {
        'halt_loop: loop {
            while let Some(event) = self.bus.scheduler.pop_current() {
                match event.tag {
                    EventTag::VBlank => {
                        self.bus.ppu.vblank(&mut self.bus.scheduler, &mut self.bus.interrupts);

                        // Persist the HALT state
                        self.bus.scheduler.schedule_event(EventTag::Halt, EmuTime(0));

                        break 'halt_loop true;
                    }
                    EventTag::PollInterrupt => {
                        self.cpu.poll_interrupts(&mut self.bus);

                        // CPU can disable HALT mode if `if & ie != 0`
                        if !self.bus.system_control.is_halted {
                            break 'halt_loop false;
                        }
                    }
                    EventTag::Halt => {
                        unreachable!("We shouldn't be able to HALT in a HALT!");
                    }
                    _ => {
                        self.handle_scheduled_event(event);
                    }
                }
            }

            // Keep skipping time until we exit out the halt loop.
            self.bus.scheduler.skip_to_next_event();
        }
    }

    /// Steps the CPU one instruction, and then checks for a breakpoint.
    ///
    /// # Returns
    ///
    /// `(Vblank was reached, breakpoint was hit)`
    pub fn step_instruction_debug(&mut self) -> (bool, bool) {
        let vsync = self.step_instruction();
        let next_pc = self.cpu.registers.next_pc();
        let breakpoint_hit = self.debug.breakpoints.iter().copied().any(|addr| next_pc == addr);

        if matches!(self.debug.break_at_cycle, Some(cycle) if cycle <= self.bus.scheduler.current_time.0) {
            self.debug.break_at_cycle = None;
            (vsync, true)
        } else {
            (vsync, breakpoint_hit)
        }
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

    pub fn frame_buffer(&mut self) -> &mut RgbaFrame {
        self.bus.ppu.frame_buffer()
    }
}

fn vec_to_bios_data(data: Option<Vec<u8>>) -> Box<BiosData> {
    let data = data.unwrap_or_else(|| vec![0; std::mem::size_of::<BiosData>()]);
    Box::try_from(data.into_boxed_slice()).unwrap()
}
