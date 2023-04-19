use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;
use crate::emulator::{GBAEmulator, MemoryAddress};

// Re-export registers which *shouldn't* be part of the public API, but for debugging purposes will be.
pub use crate::emulator::bus::interrupts::{InterruptEnable, InterruptMasterEnable, InterruptRequestFlags};
pub use crate::emulator::ppu::registers::*;
use crate::scheduler::EmuTime;

/// A reference to the [GBAEmulator] which has special access to internal state for the sake of acquiring debug information.
#[repr(transparent)]
pub struct DebugEmulator<'a>(pub &'a mut GBAEmulator);

impl<'a> DebugEmulator<'a> {
    pub fn cpu(&mut self) -> &mut CPU {
        &mut self.0.cpu
    }

    pub fn bus(&mut self) -> &mut Bus {
        &mut self.0.bus
    }

    pub fn debug_info(&mut self) -> &mut EmuDebugState {
        &mut self.0.debug
    }

    pub fn bus_and_cpu(&mut self) -> (&mut Bus, &mut CPU) {
        (&mut self.0.bus, &mut self.0.cpu)
    }
}

#[derive(Clone, Debug)]
pub enum Breakpoint {
    /// A breakpoint set for a particular address in memory.
    /// Will remain even after being hit.
    Address(MemoryAddress),
    /// A breakpoint for a particular point in time.
    ///
    /// Will be removed once hit.
    Cycle(EmuTime),
}

pub struct EmuDebugState {
    pub breakpoints: Vec<MemoryAddress>,
    pub break_at_cycle: Option<u64>,
    pub last_hit_breakpoint: Option<Breakpoint>,
}
