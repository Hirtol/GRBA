use crate::cartridge::Cartridge;
use crate::emulator::MemoryAddress;
use crate::scheduler::Scheduler;

mod ram;

pub struct Bus {
    ram: ram::WorkRam,
    rom: Cartridge,
    scheduler: Scheduler,
}

impl Bus {
    pub fn new(rom: Cartridge) -> Self {
        Self {
            ram: ram::WorkRam::new(),
            rom,
            scheduler: Scheduler::new(),
        }
    }

    pub fn read_32(&mut self, addr: MemoryAddress) -> u32 {
        match Self::get_mem_range(addr) {
            0 => todo!("BIOS READ"),
            2 => self.ram.read_board_32(addr),
            3 => self.ram.read_chip_32(addr),
            4 => todo!("IO READ"),
            5 => todo!("BG/OBJ READ"),
            6 => todo!("VRAM READ"),
            7 => todo!("OAM READ"),
            8 | 9 => todo!("ROM READ 1"),
            0xA | 0xB => todo!("ROM READ 2"),
            0xC | 0xD => todo!("ROM READ 3"),
            0xE | 0xF => todo!("Game Pak SRAM"),
            _ => todo!("Not implemented mem range!"),
        }
    }

    pub fn write_32(&mut self, addr: MemoryAddress, data: u32) {}

    pub fn read_16(&mut self, addr: MemoryAddress) -> u16 {
        0
    }

    pub fn write_16(&mut self, addr: MemoryAddress, data: u16) {}

    #[inline(always)]
    fn get_mem_range(addr: MemoryAddress) -> u32 {
        addr >> 24
    }
}
