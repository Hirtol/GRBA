use crate::emulator::ppu::registers::{LcdControl, LcdStatus, VerticalCounter, GREEN_SWAP_START};
use crate::emulator::MemoryAddress;

pub const LCD_IO_START: MemoryAddress = 0x0400_0000;
pub const LCD_IO_END: MemoryAddress = 0x4000056;
pub const DISPLAY_WIDTH: u32 = 240;
pub const DISPLAY_HEIGHT: u32 = 160;
pub const VRAM_SIZE: usize = 96 * 1024;

// 15 bit colour
// 96KB of VRAM
// 256 BG palette and 256 OBJ palette
// Transparency defined (RGBA)
// 8x8 tiles
// has direct bitmap modes
// 128 sprites can be on screen at the same time
// Sprites can go up to 64x64 (not useful)
// 6 video modes:
// * Mode 0..=2: Tiles modes
// * Mode 3..=5: Bitmap modes

mod registers;

#[derive(Debug, Clone)]
pub struct PPU {
    vram: Box<[u8; VRAM_SIZE]>,
    control: LcdControl,
    /// Not emulated
    green_swap: u16,
    status: LcdStatus,
    vertical_counter: VerticalCounter,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            vram: crate::box_array![0; VRAM_SIZE],
            control: LcdControl::new(),
            status: LcdStatus::new(),
            vertical_counter: VerticalCounter::new(),
            green_swap: 0,
        }
    }

    pub fn read_vram(&mut self, address: MemoryAddress) -> u8 {
        unimplemented!()
    }

    pub fn write_vram(&mut self, address: MemoryAddress, value: u8) {
        unimplemented!()
    }

    #[inline]
    pub fn read_io(&mut self, address: MemoryAddress) -> u8 {
        let addr = address as usize;
        let address = address - LCD_IO_START;
        match address {
            0x0..=0x1 => self.control.into_bytes()[addr % 2] as u8,
            0x2..=0x3 => self.green_swap.to_le_bytes()[addr % 2] as u8,
            0x4..=0x5 => self.status.into_bytes()[addr % 2] as u8,
            0x6..=0x7 => self.vertical_counter.into_bytes()[addr % 2] as u8,
            _ => todo!(),
        }
    }

    #[inline]
    pub fn write_io(&mut self, address: MemoryAddress, value: u8) {
        unimplemented!()
    }
}
