use crate::emulator::MemoryAddress;
use crate::scheduler::{EmuTime, EventTag, Scheduler};
use modular_bitfield::bitfield;
use modular_bitfield::prelude::{B1, B2, B7};

pub const WAIT_CNT_START: MemoryAddress = 0x0400_0204;
pub const WAIT_CNT_END: MemoryAddress = 0x0400_0207;
pub const POST_BOOT_FLAG_ADDR: MemoryAddress = 0x0400_0300;
pub const HALT_CNT_ADDR: MemoryAddress = 0x0400_0301;

pub struct GbaSystemControl {
    wait_control: WaitstateControl,
    post_boot: PostBootFlag,
    halt_control: HaltControl,

    pub is_halted: bool,
}

impl GbaSystemControl {
    pub fn new() -> Self {
        GbaSystemControl {
            wait_control: WaitstateControl::new(),
            post_boot: PostBootFlag::new(),
            halt_control: HaltControl::new(),
            is_halted: false,
        }
    }

    #[inline(always)]
    pub fn read_wait_cnt(&self, address: MemoryAddress) -> u8 {
        self.wait_control.to_le_bytes()[(address - WAIT_CNT_START) as usize]
    }

    #[inline(always)]
    pub fn read_post_boot(&self) -> u8 {
        self.post_boot.into()
    }

    #[inline]
    pub fn write_wait_cnt(&mut self, address: MemoryAddress, value: u8) {
        let addr = (address - WAIT_CNT_START) as usize;
        self.wait_control.update_byte_le(addr, value);
    }

    #[inline]
    pub fn write_post_flag(&mut self, value: u8) {
        self.post_boot = value.into();
    }

    /// Upon writes to `Halt Control` the CPU is either stopped or halted.
    ///
    /// This will schedule a `Halt` event onto the scheduler, where we'll skip until the CPU is no longer halted.
    #[inline]
    pub fn write_halt_control(&mut self, value: u8, scheduler: &mut Scheduler) -> HaltType {
        self.halt_control = value.into();

        if self.halt_control.power_down_mode() {
            HaltType::Stop
        } else {
            // Schedule our halt event
            scheduler.schedule_event(EventTag::Halt, EmuTime(0));

            self.is_halted = true;
            HaltType::Halt
        }
    }
}

pub enum HaltType {
    Halt,
    Stop,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct WaitstateControl {
    pub sram_wait_control: B2,
    pub wait_0_first_access: B2,
    pub wait_0_second_access: bool,
    pub wait_1_first_access: B2,
    pub wait_1_second_access: bool,
    pub wait_2_first_access: B2,
    pub wait_2_second_access: bool,
    pub phi_terminal_output: B2,
    #[skip]
    _unused: B1,
    /// (Pipe) (0=Disable, 1=Enable)
    game_pak_prefetch_buffer: bool,
    /// (Read Only) (0=GBA, 1=CGB)
    game_pak_type_flag: bool,
    #[skip]
    unused: u16,
}

/// After initial reset, the GBA BIOS initializes the register to `0x1`,
/// and any further execution of the Reset vector (`0x00000000`) will pass control to the Debug vector (`0x0000001C`)
/// when sensing the register to be still set to `0x1`.
#[bitfield(bits = 8)]
#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct PostBootFlag {
    /// (0=First, 1=Further)
    pub first_boot_flag: bool,
    #[skip]
    unused: B7,
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct HaltControl {
    #[skip]
    unused: B7,
    /// (0=Halt, 1=Stop)
    pub power_down_mode: bool,
}
