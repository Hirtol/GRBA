use crate::emulator::ppu::{OAM_RAM_SIZE, PALETTE_RAM_SIZE, PPU, VRAM_SIZE};
use crate::emulator::MemoryAddress;
use crate::utils::ModularBitUpdate;

pub const PALETTE_START: MemoryAddress = 0x0500_0000;
pub const PALETTE_END: MemoryAddress = 0x0500_03FF;
pub const LCD_VRAM_START: MemoryAddress = 0x0600_0000;
pub const LCD_VRAM_END: MemoryAddress = 0x0601_7FFF;
pub const OAM_START: MemoryAddress = 0x0700_0000;
pub const OAM_END: MemoryAddress = 0x0700_03FF;
pub const LCD_IO_START: MemoryAddress = 0x0400_0000;
pub const LCD_IO_END: MemoryAddress = 0x4000056;

impl PPU {
    #[inline]
    pub fn read_io(&mut self, address: MemoryAddress) -> u8 {
        let addr = address as usize;
        // Note that IO is not mirrored, therefore a subtract instead of a modulo
        let address = address - LCD_IO_START;

        match address {
            0x0..=0x1 => self.control.to_le_bytes()[addr % 2] as u8,
            0x2..=0x3 => self.green_swap.to_le_bytes()[addr % 2] as u8,
            0x4..=0x5 => self.status.to_le_bytes()[addr % 2] as u8,
            0x6..=0x7 => self.vertical_counter.to_le_bytes()[addr % 2] as u8,
            0x8..=0xF => self.bg_control[(addr % 8) / 2].to_le_bytes()[addr % 2] as u8,
            _ => todo!(),
        }
    }

    #[inline]
    pub fn write_io(&mut self, address: MemoryAddress, value: u8) {
        let addr = address as usize;
        // Note that IO is not mirrored, therefore a subtract instead of a modulo
        let address = address - LCD_IO_START;
        match address {
            0x0..=0x1 => self.control.update_byte_le(addr % 2, value),
            0x2..=0x3 => self.green_swap &= (value as u16) << ((addr % 2) * 8) as u16,
            0x4..=0x5 => self.status.update_byte_le(addr % 2, value),
            0x6..=0x7 => {
                // Vertical counter is read only
            }
            0x8..=0xF => self.bg_control[(addr % 8) / 2].update_byte_le(addr % 2, value),
            _ => todo!(),
        }
    }

    #[inline]
    pub fn read_palette(&self, address: MemoryAddress) -> u8 {
        let addr = address as usize % PALETTE_RAM_SIZE;

        self.palette_ram[addr]
    }

    #[inline]
    pub fn write_palette(&mut self, address: MemoryAddress, value: u8) {
        // When writing to palette ram with only a u8 the value is written to both the upper and lower bytes.
        let final_value = ((value as u16) << 8) | value as u16;

        self.write_palette_16(address, final_value);
    }

    #[inline]
    pub fn write_palette_16(&mut self, address: MemoryAddress, value: u16) {
        let addr = address as usize % PALETTE_RAM_SIZE;
        let data = value.to_le_bytes();
        // Better assembly
        assert!(addr < (PALETTE_RAM_SIZE - 1));

        self.palette_ram[addr] = data[0];
        self.palette_ram[addr + 1] = data[1];
    }

    #[inline]
    pub fn read_vram(&mut self, address: MemoryAddress) -> u8 {
        //TODO: Vram mirroring is awkward at 64KB + 32KB + 32KB, where the 32KB are mirrors of each other.
        let addr = (address - LCD_VRAM_START) as usize;

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
        let addr = address as usize % VRAM_SIZE;
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
