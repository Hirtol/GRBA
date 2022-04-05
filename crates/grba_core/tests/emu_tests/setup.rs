use grba_core::emulator::cartridge::Cartridge;
use grba_core::emulator::GBAEmulator;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

pub fn get_emu(rom: impl AsRef<Path>) -> GBAEmulator {
    let rom_path = get_asset_dir().join(rom);
    let rom = std::fs::read(rom_path).expect("Could not find the provided ROM");
    let ram = Box::new(MemoryRam {
        data: grba_core::box_array![0u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE],
    });

    let cartridge = Cartridge::new(rom, ram);
    GBAEmulator::new(cartridge, Default::default())
}

/// Return the `tests/assets/` directory.
pub fn get_asset_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.join("tests").join("assets")
}

struct MemoryRam {
    data: Box<[u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE]>,
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
