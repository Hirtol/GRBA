use crate::emulator::MemoryAddress;
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

    pub fn raise_interrupt(&mut self, interrupt: Interrupts) {
        todo!()
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum Interrupts {
    GamePak,
    Keypad,
    DMA3,
    DMA2,
    DMA1,
    DMA0,
    Serial,
    Timer3,
    Timer2,
    Timer1,
    Timer0,
    VCounter,
    Hblank,
    Vblank,
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
    game_pak: bool,
    keypad: bool,
    dma_3: bool,
    dma_2: bool,
    dma_1: bool,
    dma_0: bool,
    serial_communication: bool,
    timer_3: bool,
    timer_2: bool,
    timer_1: bool,
    timer_0: bool,
    vcounter_match: bool,
    hblank: bool,
    vblank: bool,
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
    game_pak: bool,
    keypad: bool,
    dma_3: bool,
    dma_2: bool,
    dma_1: bool,
    dma_0: bool,
    serial_communication: bool,
    timer_3: bool,
    timer_2: bool,
    timer_1: bool,
    timer_0: bool,
    vcounter_match: bool,
    hblank: bool,
    vblank: bool,
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
    interrupt_enable: bool,
}

crate::bitfield_update!(InterruptEnable, InterruptRequestFlags, InterruptMasterEnable);
