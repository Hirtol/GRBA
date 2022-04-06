use crate::emulator::bus::helpers::ReadType;
use crate::emulator::{AlignedAddress, MemoryAddress};
use std::fmt::{Debug, Formatter};

/// 3 cycles for access for u8, u16.
/// 6 cycles for access for u32
pub const ON_BOARD_RAM_SIZE: usize = 256 * 1024;
/// 1 cycle for access for u8, u16, u32
pub const ON_CHIP_RAM_SIZE: usize = 32 * 1024;

pub const ON_BOARD_RAM_START: usize = 0x0200_0000;
pub const ON_CHIP_RAM_START: usize = 0x0300_0000;
pub const ON_BOARD_RAM_END: usize = 0x0203_FFFF;
pub const ON_CHIP_RAM_END: usize = 0x0300_7FFF;

pub struct WorkRam {
    /// Slow RAM on board (256KB)
    board: Box<[u8; ON_BOARD_RAM_SIZE]>,
    /// Fast RAM on chip (32KB)
    chip: Box<[u8; ON_CHIP_RAM_SIZE]>,
}

impl WorkRam {
    pub fn new() -> WorkRam {
        WorkRam {
            board: crate::box_array![0; ON_BOARD_RAM_SIZE],
            chip: crate::box_array![0; ON_CHIP_RAM_SIZE],
        }
    }

    #[inline(always)]
    pub fn read_board<T: 'static + ReadType>(&self, addr: AlignedAddress) -> T {
        let addr = Self::board_addr_to_index(addr);

        if crate::is_same_type!(T, u8) {
            T::from_le_bytes(&[self.board[addr]])
        } else if crate::is_same_type!(T, u16) {
            T::from_le_bytes(&self.board[addr..addr.wrapping_add(2)])
        } else if crate::is_same_type!(T, u32) {
            T::from_le_bytes(&self.board[addr..addr.wrapping_add(4)])
        } else {
            unreachable!("Unsupported type");
        }
    }

    #[inline(always)]
    pub fn read_chip<T: 'static + ReadType>(&self, addr: AlignedAddress) -> T {
        let addr = Self::chip_addr_to_index(addr);

        let bytes = if crate::is_same_type!(T, u8) {
            &self.chip[addr..=addr]
        } else if crate::is_same_type!(T, u16) {
            &self.chip[addr..addr.wrapping_add(2)]
        } else if crate::is_same_type!(T, u32) {
            &self.chip[addr..addr.wrapping_add(4)]
        } else {
            unreachable!("Unsupported type");
        };

        T::from_le_bytes(bytes)
    }

    #[inline(always)]
    pub fn write_board(&mut self, addr: MemoryAddress, value: u8) {
        self.board[Self::board_addr_to_index(addr)] = value;
    }

    #[inline(always)]
    pub fn write_chip(&mut self, addr: MemoryAddress, value: u8) {
        self.chip[Self::chip_addr_to_index(addr)] = value;
    }

    #[inline(always)]
    pub fn write_board_16(&mut self, addr: MemoryAddress, value: u16) {
        let addr = Self::board_addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.board[addr] = bytes[0];
        self.board[addr + 1] = bytes[1];
    }

    #[inline(always)]
    pub fn write_chip_16(&mut self, addr: MemoryAddress, value: u16) {
        let addr = Self::chip_addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.chip[addr] = bytes[0];
        self.chip[addr + 1] = bytes[1];
    }

    #[inline(always)]
    pub fn write_board_32(&mut self, addr: MemoryAddress, value: u32) {
        let addr = Self::board_addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.board[addr] = bytes[0];
        self.board[addr + 1] = bytes[1];
        self.board[addr + 2] = bytes[2];
        self.board[addr + 3] = bytes[3];
    }

    #[inline(always)]
    pub fn write_chip_32(&mut self, addr: MemoryAddress, value: u32) {
        let addr = Self::chip_addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.chip[addr] = bytes[0];
        self.chip[addr + 1] = bytes[1];
        self.chip[addr + 2] = bytes[2];
        self.chip[addr + 3] = bytes[3];
    }

    #[inline(always)]
    const fn board_addr_to_index(addr: MemoryAddress) -> usize {
        // Accesses are mirrored across the range 0x0203_FFFF - 0x0200_0000
        addr as usize & (ON_BOARD_RAM_END - ON_BOARD_RAM_START)
    }

    #[inline(always)]
    const fn chip_addr_to_index(addr: MemoryAddress) -> usize {
        // Accesses are mirrored across the range 0x03007FFF - 0x03000000
        addr as usize & (ON_CHIP_RAM_END - ON_CHIP_RAM_START)
    }
}

impl Default for WorkRam {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for WorkRam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorkRam {{ on_board: ELIDED, on_chip: ELIDED }}")
    }
}

#[cfg(test)]
mod tests {
    use crate::emulator::bus::ram::WorkRam;

    #[test]
    fn test_mem_access() {
        let mut ram = WorkRam::new();

        // Ensure our basic read and writes work correctly on board RAM
        ram.write_board_32(0x02000000, 0xFEED_BEEF);

        assert_eq!(ram.read_board::<u32>(0x02000000), 0xFEED_BEEF);

        // Ensure the byte order is Little Endian
        assert_eq!(ram.read_board::<u16>(0x02000000), 0xBEEF);
        assert_eq!(ram.read_board::<u8>(0x02000000), 0xEF);

        // Ensure the same works for the chip RAM
        ram.write_chip_32(0x03000000, 0xFEED_BEEF);

        assert_eq!(ram.read_chip::<u32>(0x03000000), 0xFEED_BEEF);
        assert_eq!(ram.read_chip::<u16>(0x03000000), 0xBEEF);
        assert_eq!(ram.read_chip::<u8>(0x03000000), 0xEF);
    }
}
