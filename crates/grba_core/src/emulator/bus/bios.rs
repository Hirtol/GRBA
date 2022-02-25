use crate::emulator::cpu::CPU;
use crate::emulator::MemoryAddress;

pub type BiosData = [u8; BIOS_SIZE];

pub const BIOS_SIZE: usize = 16 * 1024;
pub const BIOS_REGION_START: u32 = 0x0;
pub const BIOS_REGION_END: u32 = 0x3FFF;

/// Handles reads and data for the BIOS region of memory
pub struct GbaBios {
    data: Box<BiosData>,
    latest_read_instr: u32,
}

impl GbaBios {
    pub fn new(data: Box<BiosData>) -> GbaBios {
        GbaBios {
            latest_read_instr: u32::from_le_bytes(data[0xE4..0xE4 + 4].try_into().unwrap()),
            data,
        }
    }

    pub fn read_32(&mut self, addr: MemoryAddress, cpu: &CPU) -> u32 {
        let pc_in_bios = Self::is_in_bios_region(cpu.registers.pc());
        if pc_in_bios {
            let addr = addr as usize;

            let read_opcode = u32::from_le_bytes(self.data[addr..addr + 4].try_into().unwrap());
            self.latest_read_instr = read_opcode;

            read_opcode
        } else {
            self.latest_read_instr
        }
    }

    pub fn read(&mut self, addr: MemoryAddress, cpu: &CPU) -> u8 {
        let pc_in_bios = Self::is_in_bios_region(cpu.registers.pc());

        if pc_in_bios {
            let addr = addr as usize;

            let read_byte = self.data[addr];
            // Reading from bios, so we should update the latest read opcode
            let mut current_opcode: [u8; 4] = self.latest_read_instr.to_le_bytes();
            current_opcode[(addr % 4) as usize] = read_byte;

            self.latest_read_instr = u32::from_le_bytes(current_opcode);

            read_byte
        } else {
            self.latest_read_instr.to_le_bytes()[addr as usize % 4]
        }
    }

    pub fn is_in_bios_region(addr: MemoryAddress) -> bool {
        addr <= BIOS_REGION_END
    }
}
