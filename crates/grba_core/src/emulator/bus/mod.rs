use crate::emulator::cartridge::Cartridge;
use crate::emulator::MemoryAddress;
use crate::scheduler::Scheduler;

mod ram;

pub struct Bus {
    ram: ram::WorkRam,
    rom: Cartridge,
    pub scheduler: Scheduler,
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
        // Temporary implementation for ease of writing.
        // In the future for performance sake we should implement an individual match for each variant, possibly.
        u32::from_le_bytes([
            self.read(addr),
            self.read(addr.wrapping_add(1)),
            self.read(addr.wrapping_add(2)),
            self.read(addr.wrapping_add(3)),
        ])
    }

    pub fn read_16(&mut self, addr: MemoryAddress) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr.wrapping_add(1))])
    }

    pub fn read(&mut self, addr: MemoryAddress) -> u8 {
        match Self::get_mem_range(addr) {
            0 => todo!("BIOS READ"),
            2 => self.ram.read_board(addr),
            3 => self.ram.read_chip(addr),
            4 => todo!("IO READ"),
            5 => todo!("BG/OBJ READ"),
            6 => todo!("VRAM READ"),
            7 => todo!("OAM READ"),
            8 | 9 => self.rom.read(addr),
            0xA | 0xB => todo!("ROM READ 2"),
            0xC | 0xD => todo!("ROM READ 3"),
            0xE | 0xF => todo!("Game Pak SRAM"),
            _ => todo!("Not implemented mem range!"),
        }
    }

    pub fn write_32(&mut self, addr: MemoryAddress, data: u32) {
        let data: [u8; 4] = data.to_le_bytes();
        self.write(addr, data[0]);
        self.write(addr.wrapping_add(1), data[1]);
        self.write(addr.wrapping_add(2), data[2]);
        self.write(addr.wrapping_add(3), data[3]);
    }

    pub fn write_16(&mut self, addr: MemoryAddress, data: u16) {
        let data: [u8; 2] = data.to_le_bytes();
        self.write(addr, data[0]);
        self.write(addr.wrapping_add(1), data[1]);
    }

    pub fn write(&mut self, addr: MemoryAddress, data: u8) {
        match Self::get_mem_range(addr) {
            0 => todo!("BIOS WRITE"),
            2 => self.ram.write_board(addr, data),
            3 => self.ram.write_chip(addr, data),
            4 => {
                crate::cpu_log!("IO WRITE: {:#X}", data);
            }
            5 => {
                crate::cpu_log!("BG/OBJ WRITE: {:#X}", data);
            }
            6 => todo!("VRAM WRITE"),
            7 => todo!("OAM WRITE"),
            8 | 9 => todo!("ROM WRITE 1"),
            0xA | 0xB => todo!("ROM WRITE 2"),
            0xC | 0xD => todo!("ROM WRITE 3"),
            0xE | 0xF => self.rom.write_sram(addr, data),
            _ => todo!("Not implemented mem range!"),
        }
    }

    #[inline(always)]
    fn get_mem_range(addr: MemoryAddress) -> u32 {
        addr >> 24
    }
}
