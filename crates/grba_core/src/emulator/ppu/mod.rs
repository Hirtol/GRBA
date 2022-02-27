use crate::emulator::MemoryAddress;

pub const LCD_IO_START: MemoryAddress = 0x4000000;
pub const LCD_IO_END: MemoryAddress = 0x4000056;
pub const DISPLAY_WIDTH: u32 = 240;
pub const DISPLAY_HEIGHT: u32 = 160;
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

#[derive(Debug, Clone, Copy)]
pub struct PPU {}

impl PPU {
    pub fn new() -> Self {
        PPU {}
    }

    pub fn read_vram(&mut self, address: MemoryAddress) -> u8 {
        unimplemented!()
    }

    pub fn write_vram(&mut self, address: MemoryAddress, value: u8) {
        unimplemented!()
    }

    #[inline]
    pub fn read_io(&mut self, address: MemoryAddress) -> u8 {
        unimplemented!()
    }

    #[inline]
    pub fn write_io(&mut self, address: MemoryAddress, value: u8) {
        unimplemented!()
    }
}
