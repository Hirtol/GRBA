use crate::emulator::bus::bios::GbaBios;
use crate::emulator::cartridge::Cartridge;
use crate::emulator::cpu::CPU;
use crate::emulator::MemoryAddress;
use crate::scheduler::Scheduler;

use crate::emulator::bus::interrupts::{InterruptManager, IE_END, IE_START, IF_END, IF_START, IME_END, IME_START};
use crate::emulator::ppu::{LCD_IO_END, LCD_IO_START, PPU};
use crate::utils::ModularBitUpdate;
pub use bios::BiosData;

mod bios;
pub mod interrupts;
mod ram;

pub struct Bus {
    bios: GbaBios,
    rom: Cartridge,
    pub interrupts: InterruptManager,
    ram: ram::WorkRam,
    ppu: PPU,
    pub scheduler: Scheduler,
}

impl Bus {
    pub fn new(rom: Cartridge, bios: Box<BiosData>) -> Self {
        Self {
            ram: ram::WorkRam::new(),
            rom,
            bios: GbaBios::new(bios),
            ppu: PPU::new(),
            scheduler: Scheduler::new(),
            interrupts: InterruptManager::new(),
        }
    }

    pub fn read_32(&mut self, addr: MemoryAddress, cpu: &CPU) -> u32 {
        // Temporary implementation for ease of writing.
        // In the future for performance sake we should implement an individual match for each variant, possibly.
        u32::from_le_bytes([
            self.read(addr, cpu),
            self.read(addr.wrapping_add(1), cpu),
            self.read(addr.wrapping_add(2), cpu),
            self.read(addr.wrapping_add(3), cpu),
        ])
    }

    pub fn read_16(&mut self, addr: MemoryAddress, cpu: &CPU) -> u16 {
        u16::from_le_bytes([self.read(addr, cpu), self.read(addr.wrapping_add(1), cpu)])
    }

    pub fn read(&mut self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        crate::cpu_log!("bus-logging"; "Reading from {:#X}", addr);
        match Self::get_mem_range(addr) {
            0 if GbaBios::is_in_bios_region(addr) => self.bios.read(addr, cpu),
            0 => self.open_bus_read(addr, cpu),
            2 => self.ram.read_board(addr),
            3 => self.ram.read_chip(addr),
            4 => self.read_io(addr, cpu),
            5 => self.ppu.read_palette(addr),
            6 => self.ppu.read_vram(addr),
            7 => self.ppu.read_oam(addr),
            8 | 9 => self.rom.read(addr),
            0xA | 0xB => todo!("ROM READ 2"),
            0xC | 0xD => todo!("ROM READ 3"),
            0xE | 0xF => todo!("Game Pak SRAM"),
            _ => self.open_bus_read(addr, cpu),
        }
    }

    pub fn write_32(&mut self, addr: MemoryAddress, data: u32) {
        let data: [u8; 4] = data.to_le_bytes();

        self.write_16(addr, u16::from_le_bytes([data[0], data[1]]));
        self.write_16(addr.wrapping_add(2), u16::from_le_bytes([data[2], data[3]]));
    }

    pub fn write_16(&mut self, addr: MemoryAddress, data: u16) {
        match Self::get_mem_range(addr) {
            5 => self.ppu.write_palette_16(addr, data),
            6 => self.ppu.write_vram_16(addr, data),
            7 => self.ppu.write_oam_16(addr, data),
            _ => {
                let data: [u8; 2] = data.to_le_bytes();
                self.write(addr, data[0]);
                self.write(addr.wrapping_add(1), data[1]);
            }
        }
    }

    pub fn write(&mut self, addr: MemoryAddress, data: u8) {
        crate::cpu_log!("bus-logging"; "Writing to {:#X} - Value: {:#X}", addr, data);
        match Self::get_mem_range(addr) {
            0 => todo!("BIOS WRITE"),
            2 => self.ram.write_board(addr, data),
            3 => self.ram.write_chip(addr, data),
            4 => self.write_io(addr, data),
            5 => self.ppu.write_palette(addr, data),
            6 => self.ppu.write_vram(addr, data),
            7 => {
                // 8 Bit OAM writes are ignored
                crate::cpu_log!("bus-logging"; "Ignored 8 bit OAM write to address: {:#X} with value: {}", addr, data)
            }
            8 | 9 => todo!("ROM WRITE 1"),
            0xA | 0xB => todo!("ROM WRITE 2"),
            0xC | 0xD => todo!("ROM WRITE 3"),
            0xE | 0xF => self.rom.write_sram(addr, data),
            _ => todo!("Not implemented mem range!"),
        }
    }

    #[inline]
    pub fn read_io(&mut self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        match addr {
            LCD_IO_START..=LCD_IO_END => self.ppu.read_io(addr),
            IE_START..=IE_END => self.interrupts.read_ie(addr),
            IF_START..=IF_END => self.interrupts.read_if(addr),
            IME_START..=IME_END => self.interrupts.read_ime(addr),
            _ => todo!("IO READ"),
        }
    }

    #[inline]
    pub fn write_io(&mut self, addr: MemoryAddress, data: u8) {
        match addr {
            LCD_IO_START..=LCD_IO_END => self.ppu.write_io(addr, data),
            IE_START..=IE_END => self.interrupts.enable.update_byte((addr % 2) as usize, data),
            IF_START..=IF_END => self.interrupts.write_if(addr, data, &mut self.scheduler),
            IME_START..=IME_END => self.interrupts.master_enable.update_byte((addr % 2) as usize, data),
            _ => todo!("IO Write"),
        }
    }

    /// Unused memory regions return the latest pre-fetched opcode.
    #[inline(always)]
    fn open_bus_read_32(&self, cpu: &CPU) -> u32 {
        // Open bus read, return prefetched Opcode
        //TODO: Handle THUMB mode special cases
        cpu.pipeline[2]
    }

    #[inline(always)]
    fn open_bus_read(&self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        self.open_bus_read_32(cpu).to_le_bytes()[addr as usize % 4]
    }

    #[inline(always)]
    fn get_mem_range(addr: MemoryAddress) -> u32 {
        // TODO: Upper four bits of the address bus are unused, should we mask them off?
        addr >> 24
    }
}
