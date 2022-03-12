use crate::emulator::MemoryAddress;
use crate::scheduler::{EmuTime, EventTag, Scheduler};
use modular_bitfield_msb::prelude::*;

pub const IE_START: MemoryAddress = 0x04000200;
pub const IE_END: MemoryAddress = 0x04000201;
pub const IF_START: MemoryAddress = 0x04000202;
pub const IF_END: MemoryAddress = 0x04000203;
pub const IME_START: MemoryAddress = 0x04000208;
pub const IME_END: MemoryAddress = 0x0400020B;

#[derive(Debug)]
pub struct InterruptManager {
    pub master_enable: InterruptMasterEnable,
    pub enable: InterruptEnable,
    pub flags: InterruptRequestFlags,
}

impl InterruptManager {
    pub fn new() -> Self {
        InterruptManager {
            master_enable: InterruptMasterEnable::new(),
            enable: InterruptEnable::new(),
            flags: InterruptRequestFlags::new(),
        }
    }

    pub fn read_ie(&self, address: MemoryAddress) -> u8 {
        self.enable.into_bytes()[(address - IE_START) as usize]
    }

    pub fn read_if(&self, address: MemoryAddress) -> u8 {
        self.flags.into_bytes()[(address - IF_START) as usize]
    }

    pub fn read_ime(&self, address: MemoryAddress) -> u8 {
        self.master_enable.into_bytes()[(address - IME_START) as usize]
    }

    pub fn write_if(&mut self, address: MemoryAddress, value: u8, scheduler: &mut Scheduler) {
        let current_value = self.read_if(address);
        // By writing a `1` to a bit that was already set, you indicate the interrupt has been handled.
        let new_value = current_value & !value;

        self.flags.update_byte((address % 2) as usize, new_value);

        // Since a potential interrupt could've been left unhandled it's necessary to immediately check for more interrupts.
        scheduler.schedule_event(EventTag::PollInterrupt, EmuTime(0));
    }

    /// Schedule an interrupt to be checked by the CPU.
    ///
    /// Note that if the corresponding bit in `IE` is not set, the interrupt will not be handled by the CPU.
    pub fn request_interrupt(&mut self, interrupt: Interrupts, scheduler: &mut Scheduler) {
        let flags_val: u16 = self.flags.into();
        let new_flag = flags_val | interrupt as u16;
        self.flags = InterruptRequestFlags::from(new_flag);

        // Schedule the interrupt to be the first thing that gets handled next.
        scheduler.schedule_event(EventTag::PollInterrupt, EmuTime(0));
    }
}

/// Interrupts that can be triggered.
///
/// Their numeric value is used to set the appropriate bit in the [InterruptRequestFlags].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
#[repr(u16)]
pub enum Interrupts {
    GamePak = 1 << 13,
    Keypad = 1 << 12,
    DMA3 = 1 << 11,
    DMA2 = 1 << 10,
    DMA1 = 1 << 9,
    DMA0 = 1 << 8,
    Serial = 1 << 7,
    Timer3 = 1 << 6,
    Timer2 = 1 << 5,
    Timer1 = 1 << 4,
    Timer0 = 1 << 3,
    VCounter = 1 << 2,
    Hblank = 1 << 1,
    Vblank = 1,
}

/// If a flag is `false` then the interrup is disabled.
#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct InterruptEnable {
    #[skip]
    unused: B2,
    /// External IRQ Source
    pub game_pak: bool,
    pub keypad: bool,
    pub dma_3: bool,
    pub dma_2: bool,
    pub dma_1: bool,
    pub dma_0: bool,
    pub serial_communication: bool,
    pub timer_3: bool,
    pub timer_2: bool,
    pub timer_1: bool,
    pub timer_0: bool,
    pub vcounter_match: bool,
    pub hblank: bool,
    pub vblank: bool,
}

/// If a flag is `true` then request interrupt
#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct InterruptRequestFlags {
    #[skip]
    unused: B2,
    /// External IRQ Source
    pub game_pak: bool,
    pub keypad: bool,
    pub dma_3: bool,
    pub dma_2: bool,
    pub dma_1: bool,
    pub dma_0: bool,
    pub serial_communication: bool,
    pub timer_3: bool,
    pub timer_2: bool,
    pub timer_1: bool,
    pub timer_0: bool,
    pub vcounter_match: bool,
    pub hblank: bool,
    pub vblank: bool,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct InterruptMasterEnable {
    #[skip]
    unused: B31,
    /// If `false` -> disable all interrupts
    ///
    /// if `true` -> See [InterruptEnableRegister] register
    pub interrupt_enable: bool,
}

crate::bitfield_update!(InterruptEnable, InterruptRequestFlags, InterruptMasterEnable);
