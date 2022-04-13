//! All debug related functionality for the PPU
use crate::emulator::ppu::{LCD_IO_START, PPU};
use crate::emulator::MemoryAddress;

impl PPU {
    /// Debug read from PPU Io memory, necessary due to the fact that most PPU registers are write only.
    #[inline]
    pub fn read_io_dbg(&mut self, address: MemoryAddress) -> u8 {
        let addr = address as usize;
        // Note that IO is not mirrored, therefore a subtract instead of a modulo
        let address = address - LCD_IO_START;
        match address {
            0x0..=0x1 => self.dispcnt.to_le_bytes()[addr % 2],
            0x2..=0x3 => self.green_swap.to_le_bytes()[addr % 2],
            0x4..=0x5 => self.dispstat.to_le_bytes()[addr % 2],
            0x6..=0x7 => self.vertical_counter.to_le_bytes()[addr % 2],
            0x8..=0x9 => self.bg_control[0].to_le_bytes()[addr % 2],
            0xA..=0xB => self.bg_control[1].to_le_bytes()[addr % 2],
            0xC..=0xD => self.bg_control[2].to_le_bytes()[addr % 2],
            0xE..=0xF => self.bg_control[3].to_le_bytes()[addr % 2],
            0x10..=0x11 => self.bg_scrolling[0][0].to_le_bytes()[addr % 2],
            0x12..=0x13 => self.bg_scrolling[1][0].to_le_bytes()[addr % 2],
            0x14..=0x15 => self.bg_scrolling[2][0].to_le_bytes()[addr % 2],
            0x16..=0x17 => self.bg_scrolling[3][0].to_le_bytes()[addr % 2],
            0x18..=0x19 => self.bg_scrolling[0][1].to_le_bytes()[addr % 2],
            0x1A..=0x1B => self.bg_scrolling[1][1].to_le_bytes()[addr % 2],
            0x1C..=0x1D => self.bg_scrolling[2][1].to_le_bytes()[addr % 2],
            0x1E..=0x1F => self.bg_scrolling[3][1].to_le_bytes()[addr % 2],
            0x20..=0x21 => self.bg_rotation_reference_bg2[0].to_le_bytes()[addr % 2],
            0x22..=0x23 => self.bg_rotation_reference_bg2[1].to_le_bytes()[addr % 2],
            0x24..=0x25 => self.bg_rotation_reference_bg2[2].to_le_bytes()[addr % 2],
            0x26..=0x27 => self.bg_rotation_reference_bg2[3].to_le_bytes()[addr % 2],
            0x28..=0x2B => self.bg_rotation_x[0].to_le_bytes()[addr % 4],
            0x2C..=0x2F => self.bg_rotation_y[0].to_le_bytes()[addr % 4],
            0x30..=0x31 => self.bg_rotation_reference_bg3[0].to_le_bytes()[addr % 2],
            0x32..=0x33 => self.bg_rotation_reference_bg3[1].to_le_bytes()[addr % 2],
            0x34..=0x35 => self.bg_rotation_reference_bg3[2].to_le_bytes()[addr % 2],
            0x36..=0x37 => self.bg_rotation_reference_bg3[3].to_le_bytes()[addr % 2],
            0x38..=0x3B => self.bg_rotation_x[1].to_le_bytes()[addr % 4],
            0x3C..=0x3F => self.bg_rotation_y[1].to_le_bytes()[addr % 4],
            0x40..=0x41 => self.window_horizontal[0].to_le_bytes()[addr % 2],
            0x42..=0x43 => self.window_horizontal[1].to_le_bytes()[addr % 2],
            0x44..=0x45 => self.window_vertical[0].to_le_bytes()[addr % 2],
            0x46..=0x47 => self.window_vertical[1].to_le_bytes()[addr % 2],
            0x48..=0x49 => self.window_control_inside.to_le_bytes()[addr % 2],
            0x4A..=0x4B => self.window_control_outside.to_le_bytes()[addr % 2],
            0x4C..=0x4F => self.mosaic_function.to_le_bytes()[addr % 4],
            0x50..=0x51 => self.special.to_le_bytes()[addr % 2],
            0x52..=0x53 => self.alpha.to_le_bytes()[addr % 2],
            0x54..=0x55 => self.brightness.to_le_bytes()[addr % 2],
            0x56 => 0xFF,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn write_vram_dbg(&mut self, address: MemoryAddress, value: u8) {
        let addr = (address & 0x1FFFF) as usize;

        self.vram[addr] = value;
    }
}
