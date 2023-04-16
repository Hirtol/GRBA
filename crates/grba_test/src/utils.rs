use std::ops::{Deref, DerefMut};

pub struct MemoryRam {
    data: Box<[u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE]>,
}

impl Default for MemoryRam {
    fn default() -> Self {
        Self {
            data: grba_core::box_array![0; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE],
        }
    }
}

impl Deref for MemoryRam {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.data
    }
}

impl DerefMut for MemoryRam {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.data
    }
}
