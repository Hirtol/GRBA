use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use clap::Parser;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;

use grba_core::emulator::frame::RgbaFrame;

use crate::config::ClapArgs;
use crate::utils::MemoryRam;

mod config;
mod formatting;
mod panics;
mod processing;
mod setup;
mod utils;

fn main() -> anyhow::Result<()> {
    let clap_args = ClapArgs::parse();
    let config = config::load_config()?;

    rayon::ThreadPoolBuilder::default()
        .num_threads(config.max_parallelism.get())
        .build_global()?;

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

    setup::setup_output_directory(&output_path)?;
    setup::setup_snapshot_directory(&snapshots)?;

    // Run the tests
    let roms_to_run = utils::list_files_with_extensions(test_roms, ".gba")?;
    let bios = std::fs::read(bios_path)?;
    let formatter = formatting::SimpleReporter::new(&output_path);

    formatter.handle_start(&roms_to_run);

    let now = Instant::now();
    let frame_results = panics::run_in_custom_handler(|| {
        roms_to_run
            .into_par_iter()
            .map(|rom| {
                let name = get_rom_fs_name(&rom);
                let runner_output = std::fs::read(&rom).context("Couldn't read ROM").and_then(|rom_data| {
                    let frames = config
                        .custom_configs
                        .get(&name)
                        .map(|conf| conf.num_frames)
                        .unwrap_or(clap_args.frames);

                    let now = Instant::now();
                    let frame = run_normal_test(&rom, rom_data, frames, &bios)?;

                    Ok(RunnerOutput {
                        rom_path: rom.clone(),
                        rom_name: name.clone(),
                        context: RunnerOutputContext {
                            time_taken: now.elapsed(),
                            frame_output: frame,
                        },
                    })
                });

                runner_output.map_err(|e| RunnerError {
                    rom_path: rom,
                    rom_name: name,
                    context: e,
                })
            })
            .progress()
            .collect::<Vec<_>>()
    });

    let results = processing::process_results(frame_results, &output_path, &snapshots);

    formatter.handle_complete_tests(&results, now.elapsed());

    Ok(())
}
#[derive(Debug)]
pub struct EmuContext<T> {
    pub rom_path: PathBuf,
    pub rom_name: String,
    pub context: T,
}

impl<T> EmuContext<T> {
    pub fn map<E, F: FnOnce(T) -> E>(self, op: F) -> EmuContext<E> {
        EmuContext {
            rom_path: self.rom_path,
            rom_name: self.rom_name,
            context: op(self.context),
        }
    }
}

pub type RunnerError = EmuContext<anyhow::Error>;
pub type RunnerOutput = EmuContext<RunnerOutputContext>;

#[derive(Debug)]
pub struct RunnerOutputContext {
    pub time_taken: Duration,
    pub frame_output: RgbaFrame,
}

pub fn run_normal_test(rom_path: &Path, rom: Vec<u8>, frames_to_run: u32, bios: &[u8]) -> anyhow::Result<RgbaFrame> {
    let out = std::panic::catch_unwind(move || {
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
    });

    match out {
        Ok(frame) => Ok(frame),
        Err(e) => Err(anyhow!(
            "Caught an emulator panic: `{}`",
            panics::correlate(&rom_path.as_os_str().to_string_lossy())
        )),
    }
}

pub fn get_rom_fs_name(path: &Path) -> String {
    path.file_stem()
        .expect("Failed to get rom stem")
        .to_string_lossy()
        .to_string()
}
