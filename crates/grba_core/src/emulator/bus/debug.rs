//! All debug related functionality for the [Bus] component.

use crate::emulator::bus::{Bus, IO_START};
use crate::emulator::cpu::CPU;
use crate::emulator::ppu::LCD_IO_END;
use crate::emulator::MemoryAddress;

impl Bus {
    #[inline]
    pub fn read_dbg(&mut self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        match Self::get_mem_range(addr) {
            4 => self.read_io_dbg(addr, cpu),
            _ => self.read(addr, cpu),
        }
    }

    #[inline]
    pub fn write_dbg(&mut self, addr: MemoryAddress, data: u8) {
        match Self::get_mem_range(addr) {
            4 => self.write_io_dbg(addr, data),
            6 => self.ppu.write_vram_dbg(addr, data),
            7 => {
                // 8 Bit OAM writes are usually ignored, for debug purposes we'll allow it
                let current_data = (self.ppu.read_oam(addr.wrapping_add(1)) as u16) << 8;
                self.ppu.write_oam_16(addr, current_data | data as u16)
            }
            _ => self.write(addr, data),
        }
    }

    #[inline]
    fn read_io_dbg(&mut self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        match addr {
            IO_START..=LCD_IO_END => self.ppu.read_io_dbg(addr),
            _ => self.read_io(addr, cpu),
        }
    }

    #[inline]
    fn write_io_dbg(&mut self, addr: MemoryAddress, data: u8) {
        match addr {
            _ => self.write_io(addr, data),
        }
    }
}
