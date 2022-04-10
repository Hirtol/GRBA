use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;
use crate::emulator::{EmuDebugging, GBAEmulator};

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

    pub fn debug_info(&mut self) -> &mut EmuDebugging {
        &mut self.0.debug
    }

    pub fn bus_and_cpu(&mut self) -> (&mut Bus, &mut CPU) {
        (&mut self.0.bus, &mut self.0.cpu)
    }
}
