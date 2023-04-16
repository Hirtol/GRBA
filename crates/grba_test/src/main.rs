use anyhow::Context;
use clap::Parser;
use emu_test_runner::formatters::simple::SimpleConsoleFormatter;
use emu_test_runner::inputs::TestCandidate;
use emu_test_runner::options::EmuRunnerOptions;
use emu_test_runner::EmuTestRunner;

use grba_core::emulator::frame::RgbaFrame;

use crate::config::ClapArgs;
use crate::utils::MemoryRam;

mod config;
mod utils;

fn main() -> anyhow::Result<()> {
    let clap_args = ClapArgs::parse();
    let config = config::load_config()?;

    let test_roms = clap_args
        .test_rom_dir
        .unwrap_or(config.test_rom_dir.clone())
        .canonicalize()
        .context("Couldn't find the test rom directory")?;
    let bios_path = clap_args
        .bios
        .unwrap_or(config.bios_path.clone())
        .canonicalize()
        .context("Couldn't find the GBA Bios")?;
    let output_path = clap_args.output_path.unwrap_or(config.output_path.clone());
    let snapshots = config.snapshot_path.clone();

    let tests = TestCandidate::find_all_in_directory(test_roms, ".gba")?;
    let formatter = Box::new(SimpleConsoleFormatter::new().with_progress(tests.len() as u64));
    let options = EmuRunnerOptions {
        output_path,
        snapshot_path: snapshots,
        num_threads: clap_args.num_threads.unwrap_or(config.num_threads),
        expected_frame_width: grba_core::DISPLAY_WIDTH as usize,
        expected_frame_height: grba_core::DISPLAY_HEIGHT as usize,
    };
    let runner = EmuTestRunner::new(formatter, options)?;

    let bios = std::fs::read(bios_path)?;

    runner.run_tests(tests.into_iter(), |test, rom_data| {
        let frames_to_run = config
            .custom_configs
            .get(&test.rom_id)
            .map(|conf| conf.num_frames)
            .unwrap_or(clap_args.frames);

        let frame = run_normal_test(rom_data, frames_to_run, &bios);

        emu_test_runner::outputs::RgbaFrame(frame.as_bytes().to_vec())
    })?;

    Ok(())
}

pub fn run_normal_test(rom: Vec<u8>, frames_to_run: u32, bios: &[u8]) -> RgbaFrame {
    let emu_options = grba_core::emulator::EmuOptions {
        skip_bios: true,
        bios: Some(bios.to_owned()),
        debugging: false,
    };
    let cartridge = grba_core::emulator::cartridge::Cartridge::new(rom, Box::new(MemoryRam::default()));
    let mut emu = grba_core::emulator::GBAEmulator::new(cartridge, emu_options);

    for _ in 0..frames_to_run {
        emu.run_to_vblank();
    }

    emu.frame_buffer().clone()
}
