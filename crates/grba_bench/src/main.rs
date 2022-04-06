//! Ugly
use clap::{AppSettings, Parser, Subcommand};
use grba_core::emulator::cartridge::header::CartridgeHeader;
use grba_core::emulator::cartridge::Cartridge;
use grba_core::emulator::GBAEmulator;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(clap::Parser, Debug)]
#[clap(version, about)]
pub struct Args {
    /// The path of the ROM to benchmark
    pub rom_path: PathBuf,
    /// The amount of frames to emulate
    #[clap(short, default_value = "2000")]
    pub frames: u32,
}

fn main() {
    let args = Args::parse();
    let (mut emulator, header) = get_emu(args.rom_path);
    println!("Running {:?} for {} frames", header.game_title, args.frames);

    let start = Instant::now();

    for _ in 0..args.frames {
        emulator.run_to_vblank();
    }

    println!(
        "Executing took {:?} for a total of {} frames per second",
        start.elapsed(),
        args.frames as f64 / start.elapsed().as_secs_f64()
    );
}

pub fn get_emu(rom: impl AsRef<Path>) -> (GBAEmulator, CartridgeHeader) {
    let rom = std::fs::read(rom).expect("Could not find the provided ROM");
    let ram = Box::new(MemoryRam {
        data: grba_core::box_array![0u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE],
    });

    let cartridge = Cartridge::new(rom, ram);
    let header = cartridge.header().clone();
    (GBAEmulator::new(cartridge, Default::default()), header)
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
