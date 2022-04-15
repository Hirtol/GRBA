use crate::emulator::bus::IO_START;
use crate::emulator::ppu::{OAM_RAM_SIZE, PPU, VRAM_SIZE};
use crate::emulator::MemoryAddress;
use crate::utils::BitOps;

pub const PALETTE_START: MemoryAddress = 0x0500_0000;
pub const PALETTE_END: MemoryAddress = 0x0500_03FF;
pub const LCD_VRAM_START: MemoryAddress = 0x0600_0000;
pub const LCD_VRAM_END: MemoryAddress = 0x0601_7FFF;
pub const OAM_START: MemoryAddress = 0x0700_0000;
pub const OAM_END: MemoryAddress = 0x0700_03FF;
pub const LCD_IO_END: MemoryAddress = 0x4000056;

impl PPU {
    #[inline]
    pub fn read_io(&mut self, address: MemoryAddress) -> u8 {
        let addr = address as usize;
        // Note that IO is not mirrored, therefore a subtract instead of a modulo
        let address = address - IO_START;

        match address {
            0x0..=0x1 => self.disp_cnt.to_le_bytes()[addr % 2],
            0x2..=0x3 => self.green_swap.to_le_bytes()[addr % 2],
            0x4..=0x5 => self.disp_stat.to_le_bytes()[addr % 2],
            0x6..=0x7 => self.vertical_counter.to_le_bytes()[addr % 2],
            0x8..=0xF => self.bg_control[(addr % 8) / 2].to_le_bytes()[addr % 2],
            0x10..=0x3F => {
                // bg_scrolling is write-only, TODO: Open bus read
                0xFF
            }
            0x40..=0x47 => {
                // Window registers are write-only TODO: Open bus read
                0xFF
            }
            0x48..=0x49 => self.window_control_inside.to_le_bytes()[addr % 2],
            0x4A..=0x4B => self.window_control_outside.to_le_bytes()[addr % 2],
            0x50..=0x51 => self.bld_cnt.to_le_bytes()[addr % 2],
            0x52..=0x53 => self.alpha.to_le_bytes()[addr % 2],
            0x54..=0x55 => self.brightness.to_le_bytes()[addr % 2],
            _ => {
                // TODO: Open bus read
                crate::cpu_log!("ppu-logging"; "Unimplemented IO read at {:08X}", address);
                0xFF
            }
        }
    }

    #[inline]
    pub fn write_io(&mut self, address: MemoryAddress, value: u8) {
        let addr = address as usize;
        // Note that IO is not mirrored, therefore a subtract instead of a modulo
        let address = address - IO_START;
        match address {
            0x0..=0x1 => self.disp_cnt.update_byte_le(addr % 2, value),
            0x2..=0x3 => self.green_swap = self.green_swap.change_byte_le(addr % 2, value),
            0x4..=0x5 => self.disp_stat.update_byte_le(addr % 2, value),
            0x6..=0x7 => {
                // Vertical counter is read only
            }
            0x8..=0x9 => self.bg_control[0].update_byte_le(addr % 2, value),
            0xA..=0xB => self.bg_control[1].update_byte_le(addr % 2, value),
            0xC..=0xD => self.bg_control[2].update_byte_le(addr % 2, value),
            0xE..=0xF => self.bg_control[3].update_byte_le(addr % 2, value),
            0x10..=0x11 => self.bg_scrolling[0][0].update_byte_le(addr % 2, value),
            0x12..=0x13 => self.bg_scrolling[1][0].update_byte_le(addr % 2, value),
            0x14..=0x15 => self.bg_scrolling[2][0].update_byte_le(addr % 2, value),
            0x16..=0x17 => self.bg_scrolling[3][0].update_byte_le(addr % 2, value),
            0x18..=0x19 => self.bg_scrolling[0][1].update_byte_le(addr % 2, value),
            0x1A..=0x1B => self.bg_scrolling[1][1].update_byte_le(addr % 2, value),
            0x1C..=0x1D => self.bg_scrolling[2][1].update_byte_le(addr % 2, value),
            0x1E..=0x1F => self.bg_scrolling[3][1].update_byte_le(addr % 2, value),
            0x20..=0x21 => self.bg_rotation_reference_bg2[0].update_byte_le(addr % 2, value),
            0x22..=0x23 => self.bg_rotation_reference_bg2[1].update_byte_le(addr % 2, value),
            0x24..=0x25 => self.bg_rotation_reference_bg2[2].update_byte_le(addr % 2, value),
            0x26..=0x27 => self.bg_rotation_reference_bg2[3].update_byte_le(addr % 2, value),
            0x28..=0x2B => self.bg_rotation_x[0].update_byte_le(addr % 4, value),
            0x2C..=0x2F => self.bg_rotation_y[0].update_byte_le(addr % 4, value),
            0x30..=0x31 => self.bg_rotation_reference_bg3[0].update_byte_le(addr % 2, value),
            0x32..=0x33 => self.bg_rotation_reference_bg3[1].update_byte_le(addr % 2, value),
            0x34..=0x35 => self.bg_rotation_reference_bg3[2].update_byte_le(addr % 2, value),
            0x36..=0x37 => self.bg_rotation_reference_bg3[3].update_byte_le(addr % 2, value),
            0x38..=0x3B => self.bg_rotation_x[1].update_byte_le(addr % 4, value),
            0x3C..=0x3F => self.bg_rotation_y[1].update_byte_le(addr % 4, value),
            0x40..=0x41 => self.window_horizontal[0].update_byte_le(addr % 2, value),
            0x42..=0x43 => self.window_horizontal[1].update_byte_le(addr % 2, value),
            0x44..=0x45 => self.window_vertical[0].update_byte_le(addr % 2, value),
            0x46..=0x47 => self.window_vertical[1].update_byte_le(addr % 2, value),
            0x48..=0x49 => self.window_control_inside.update_byte_le(addr % 2, value),
            0x4A..=0x4B => self.window_control_outside.update_byte_le(addr % 2, value),
            0x4C..=0x4F => self.mosaic_function.update_byte_le(addr % 4, value),
            0x50..=0x51 => self.bld_cnt.update_byte_le(addr % 2, value),
            0x52..=0x53 => self.alpha.update_byte_le(addr % 2, value),
            0x54..=0x55 => self.brightness.update_byte_le(addr % 2, value),
            0x56 => {
                // not used
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn read_palette(&self, address: MemoryAddress) -> u8 {
        self.palette.read_palette(address)
    }

    #[inline]
    pub fn write_palette(&mut self, address: MemoryAddress, value: u8) {
        self.palette.write_palette(address, value);
    }

    #[inline]
    pub fn write_palette_16(&mut self, address: MemoryAddress, value: u16) {
        self.palette.write_palette_16(address, value);
    }

    #[inline]
    pub fn read_vram(&mut self, address: MemoryAddress) -> u8 {
        let addr = get_vram_address(address);

        self.vram[addr]
    }

    #[inline]
    pub fn write_vram(&mut self, address: MemoryAddress, value: u8) {
        // When writing to vram ram with only a u8 the value is written to both the upper and lower bytes.
        //TODO: Potentially ignore 8 bit writes to OBJ (6010000h-6017FFFh) (or 6014000h-6017FFFh in Bitmap mode)
        let final_value = ((value as u16) << 8) | value as u16;

        self.write_vram_16(address, final_value);
    }

    #[inline]
    pub fn write_vram_16(&mut self, address: MemoryAddress, value: u16) {
        let addr = get_vram_address(address);
        let data = value.to_le_bytes();
        // Better assembly
        assert!(addr < (VRAM_SIZE - 1));

        self.vram[addr] = data[0];
        self.vram[addr + 1] = data[1];
    }

    #[inline]
    pub fn read_oam(&mut self, address: MemoryAddress) -> u8 {
        // Memory is mirrored
        let addr = address as usize % OAM_RAM_SIZE;

        self.oam_ram[addr]
    }

    #[inline]
    pub fn write_oam_16(&mut self, address: MemoryAddress, value: u16) {
        let addr = address as usize % OAM_RAM_SIZE;
        let data = value.to_le_bytes();
        // Better assembly
        assert!(addr < (OAM_RAM_SIZE - 1));

        self.oam_ram[addr] = data[0];
        self.oam_ram[addr + 1] = data[1];
    }
}

#[inline(always)]
fn get_vram_address(address: MemoryAddress) -> usize {
    // VRAM mirroring is awkward at 64KB + 32KB + 32KB, where the 32KB are mirrors of each other.
    let mut addr = (address & 0x1FFFF) as usize;

    if addr >= VRAM_SIZE {
        addr -= 0x8000;
    }

    addr
}
