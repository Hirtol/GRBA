use crate::cartridge::header::CartridgeHeader;
use crate::emulator::MemoryAddress;

pub mod header;

/// Maximum of `64KB` of additional SRAM
pub const CARTRIDGE_RAM_SIZE: usize = 1024 * 64;

pub const CARTRIDGE_ROM_START: MemoryAddress = 0x08000000;
pub const CARTRIDGE_SRAM_START: MemoryAddress = 0x0E000000;

pub struct Cartridge {
    header: CartridgeHeader,
    rom: Vec<u8>,
    /// SRAM is stored in the cartridge file
    ///
    /// This is usually buffered by a backing battery, so it can be seen as a save.
    /// For now we take a [Box] here to avoid needing to specify lifetimes everywhere (as we want to be able to take
    /// a MMAP, or any byte array really). If performance turns out to be significantly worse we can always change it.
    ///
    /// TODO: Implement different sizes based on the backup ID, currently we just assume Flash.
    saved_ram: Box<dyn std::ops::DerefMut<Target = [u8]> + Send>,
}

impl Cartridge {
    pub fn new(rom: Vec<u8>, ram: Box<dyn std::ops::DerefMut<Target = [u8]> + Send>) -> Self {
        let header = CartridgeHeader::new(&rom);
        Self {
            header,
            rom,
            saved_ram: ram,
        }
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    pub fn rom(&self) -> &[u8] {
        &self.rom
    }

    /// Read the value at the provided `addr` from SRAM.
    ///
    /// Note that the ROM only has an 8-bit bus, so this should only ever return a [u8]
    pub fn read_sram(&self, addr: MemoryAddress) -> u8 {
        self.saved_ram[Self::cartridge_sram_addr_to_index(addr)]
    }

    /// Write the given `value` to the given `addr` in SRAM.
    pub fn write_sram(&mut self, addr: MemoryAddress, value: u8) {
        self.saved_ram[Self::cartridge_sram_addr_to_index(addr)] = value;
    }

    #[inline(always)]
    pub fn read(&self, addr: MemoryAddress) -> u8 {
        self.rom[Self::cartridge_rom_addr_to_index(addr)]
    }

    #[inline(always)]
    pub fn read_16(&self, addr: MemoryAddress) -> u16 {
        let addr = Self::cartridge_rom_addr_to_index(addr);

        u16::from_le_bytes((&self.rom[addr..addr + 2]).try_into().unwrap())
    }

    #[inline(always)]
    pub fn read_32(&self, addr: MemoryAddress) -> u32 {
        let addr = Self::cartridge_rom_addr_to_index(addr);

        u32::from_le_bytes((&self.rom[addr..addr + 4]).try_into().unwrap())
    }

    fn cartridge_sram_addr_to_index(addr: MemoryAddress) -> usize {
        (addr - CARTRIDGE_SRAM_START) as usize
    }

    fn cartridge_rom_addr_to_index(addr: MemoryAddress) -> usize {
        (addr - CARTRIDGE_ROM_START) as usize
    }
}
