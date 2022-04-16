use crate::emulator::MemoryAddress;

pub const OAM_RAM_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct OamRam {
    /// The raw bytes used by the emulator for storage
    oam_ram: Box<[u8; OAM_RAM_SIZE]>,
}

impl OamRam {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub const fn ram(&self) -> &[u8; OAM_RAM_SIZE] {
        &self.oam_ram
    }

    #[inline]
    pub fn read_oam(&self, address: MemoryAddress) -> u8 {
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

impl Default for OamRam {
    fn default() -> Self {
        Self {
            oam_ram: crate::box_array![0; OAM_RAM_SIZE],
        }
    }
}
