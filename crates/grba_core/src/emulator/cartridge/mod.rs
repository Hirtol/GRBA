use crate::emulator::bus::helpers::ReadType;
use crate::emulator::cartridge::header::CartridgeHeader;
use crate::emulator::{AlignedAddress, MemoryAddress};
use std::ops::{Deref, DerefMut};

pub mod header;

pub const MAX_ROM_SIZE: usize = 1024 * 1024 * 32;
/// Maximum of `64KB` of additional SRAM
pub const CARTRIDGE_RAM_SIZE: usize = 1024 * 64;

pub const CARTRIDGE_ROM_START: MemoryAddress = 0x0800_0000;
pub const CARTRIDGE_SRAM_START: MemoryAddress = 0x0E00_0000;

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
    pub fn new(mut rom: Vec<u8>, ram: Box<dyn std::ops::DerefMut<Target = [u8]> + Send>) -> Self {
        let header = CartridgeHeader::new(&rom);

        // Since games like to do out of bound reads we need to pre-emptively fill the data
        if rom.len() < MAX_ROM_SIZE {
            fill_rom_out_of_bounds(&mut rom);
        }

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

    pub fn ram(&self) -> &[u8] {
        &self.saved_ram
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

    #[inline]
    pub fn read<T: 'static + ReadType>(&self, addr: AlignedAddress) -> T {
        let addr = Self::cartridge_rom_addr_to_index(addr);

        if crate::is_same_type!(T, u8) {
            T::from_le_bytes(&[self.rom[addr]])
        } else if crate::is_same_type!(T, u16) {
            T::from_le_bytes(&self.rom[addr..addr.wrapping_add(2)])
        } else if crate::is_same_type!(T, u32) {
            T::from_le_bytes(&self.rom[addr..addr.wrapping_add(4)])
        } else {
            unreachable!("Unsupported type");
        }
    }

    #[inline(always)]
    const fn cartridge_sram_addr_to_index(addr: MemoryAddress) -> usize {
        addr as usize % CARTRIDGE_RAM_SIZE
    }

    #[inline(always)]
    const fn cartridge_rom_addr_to_index(addr: AlignedAddress) -> usize {
        addr as usize % MAX_ROM_SIZE
    }
}

#[doc(hidden)]
/// Purely used for debugging.
impl Default for Cartridge {
    fn default() -> Self {
        Cartridge {
            header: CartridgeHeader::new(&[0; 2000]),
            rom: Vec::new(),
            saved_ram: Box::new(FakeRam),
        }
    }
}

/// Fill a ROM with OoB data for reads.
///
/// Implementation translated from [open_agb](https://github.com/profi200/open_agb_firm/blob/a9fcf853bb2b21623f528ac23675c8af05180297/source/arm11/open_agb_firm.c#L119)
pub fn fill_rom_out_of_bounds(rom: &mut Vec<u8>) {
    let original_length = rom.len();
    rom.reserve(MAX_ROM_SIZE - rom.len());

    // TODO: ROM Mirroring for the NES Series? See implementation in [open_agb]
    // TODO: Use proper ROM values (Address/2 & 0xFFFF)
    rom.resize(MAX_ROM_SIZE, 0xFF);
}

struct FakeRam;

impl Deref for FakeRam {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl DerefMut for FakeRam {
    fn deref_mut(&mut self) -> &mut Self::Target {
        todo!()
    }
}
