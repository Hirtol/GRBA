#![feature(type_name_of_val)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use clap::Parser;
use image::{EncodableLayout, ImageBuffer};
use rayon::prelude::*;

use grba_core::emulator::frame::RgbaFrame;

use crate::config::ClapArgs;
use crate::utils::MemoryRam;

mod config;
mod panics;
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

    setup_output_directory(&output_path)?;
    setup_snapshot_directory(&snapshots)?;

    // Run the tests
    let roms_to_run = utils::list_files_with_extensions(&test_roms, ".gba")?;
    let bios = std::fs::read(bios_path)?;

    let frame_results = panics::run_in_custom_handler(|| {
        roms_to_run
            .into_iter()
            .par_bridge()
            .map(|rom| {
                let name = get_rom_fs_name(&rom);
                let runner_output = std::fs::read(&rom).context("Couldn't read ROM").and_then(|rom_data| {
                    let frames = config
                        .custom_configs
                        .get(&name)
                        .map(|conf| conf.num_frames)
                        .unwrap_or(clap_args.frames);

                    let now = Instant::now();
                    let out = run_normal_test(&rom, rom_data, frames, &bios)?;
                    // let frame = run_normal_test(rom_data, frames, &bios)?;

                    Ok(RunnerOutput {
                        rom_path: rom.clone(),
                        rom_name: name.clone(),
                        time_taken: now.elapsed(),
                        frame_output: out,
                    })
                });

                runner_output.map_err(|e| RunnerError {
                    rom_path: rom,
                    rom_name: name,
                    context: e,
                })
            })
            .collect::<Vec<_>>()
    });

    let results = process_results(frame_results, &output_path, &snapshots);

    println!("{:#?}", results);

    Ok(())
}

pub struct RunnerError {
    pub rom_path: PathBuf,
    pub rom_name: String,
    pub context: anyhow::Error,
}

#[derive(Debug)]
pub struct RunnerOutput {
    pub rom_path: PathBuf,
    pub rom_name: String,
    pub time_taken: Duration,
    pub frame_output: RgbaFrame,
}

impl From<RunnerError> for TestResult {
    fn from(value: RunnerError) -> Self {
        TestResult {
            rom_path: value.rom_path,
            rom_name: value.rom_name,
            time_taken: None,
            output: TestOutputType::Error { reason: value.context },
        }
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub rom_path: PathBuf,
    pub rom_name: String,
    pub time_taken: Option<Duration>,
    pub output: TestOutputType,
}

#[derive(Debug)]
pub enum TestOutputType {
    Unchanged,
    Changed {
        changed_path_dump: PathBuf,
        old_path: PathBuf,
    },
    Failure {
        failure_path: PathBuf,
        snapshot_path: PathBuf,
    },
    Passed,
    Error {
        reason: anyhow::Error,
    },
}

pub fn process_results(
    results: Vec<Result<RunnerOutput, RunnerError>>,
    output: &Path,
    snapshot_dir: &Path,
) -> Vec<TestResult> {
    results
        .into_par_iter()
        .map(|runner_output| {
            let runner_output = match runner_output {
                Ok(output) => output,
                Err(e) => return e.into(),
            };
            let lambda = || {
                let image_frame: ImageBuffer<image::Rgba<u8>, &[u8]> = if let Some(img) = image::ImageBuffer::from_raw(
                    grba_core::DISPLAY_WIDTH,
                    grba_core::DISPLAY_HEIGHT,
                    runner_output.frame_output.as_bytes(),
                ) {
                    img
                } else {
                    anyhow::bail!("Failed to turn GRBA framebuffer to dynamic image")
                };

                let result_name = format!("{}.png", &runner_output.rom_name);
                let new_path = new_path(output).join(&result_name);

                image_frame.save(&new_path)?;

                let output = if let Some(snapshot) = has_snapshot(&runner_output.rom_name, snapshot_dir) {
                    // Time to see if our snapshot is still correct
                    let snapshot_data = image::open(&snapshot)?;

                    if snapshot_data.as_bytes() != image_frame.as_bytes() {
                        let failure_path = failures_path(output).join(&result_name);
                        std::fs::copy(&new_path, &failure_path)?;

                        TestOutputType::Failure {
                            failure_path,
                            snapshot_path: snapshot,
                        }
                    } else {
                        TestOutputType::Passed
                    }
                } else {
                    // Just check if there has been *any* change at all
                    let old_path = old_path(output).join(&result_name);

                    if old_path.exists() {
                        let old_data = image::open(&old_path)?;

                        if old_data.as_bytes() != image_frame.as_bytes() {
                            let changed_path = changed_path(output).join(&result_name);
                            std::fs::copy(&new_path, &changed_path)?;

                            TestOutputType::Changed {
                                changed_path_dump: changed_path,
                                old_path,
                            }
                        } else {
                            TestOutputType::Unchanged
                        }
                    } else {
                        TestOutputType::Unchanged
                    }
                };

                Ok(output)
            };

            match lambda() {
                Ok(output) => TestResult {
                    rom_path: runner_output.rom_path,
                    rom_name: runner_output.rom_name,
                    time_taken: Some(runner_output.time_taken),
                    output,
                },
                Err(e) => TestResult {
                    rom_path: runner_output.rom_path,
                    rom_name: runner_output.rom_name,
                    time_taken: Some(runner_output.time_taken),
                    output: TestOutputType::Error { reason: e },
                },
            }
        })
        .collect()
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

/// Will clean and setup the directory structure in the output directory as follows:
///
///
/// * OUTPUT_DIR
///     * /new
///     * /old
///     * /changed
///     * /failures
pub fn setup_output_directory(output: &Path) -> anyhow::Result<()> {
    let new_dir = new_path(output);
    let old_dir = old_path(output);
    let changed_dir = changed_path(output);
    let failures = failures_path(output);

    let _ = std::fs::remove_dir_all(&old_dir);
    // Move the `new` dir to the `old`
    if new_dir.exists() {
        std::fs::rename(&new_dir, &old_dir)?;
    }

    let _ = std::fs::remove_dir_all(&changed_dir);
    let _ = std::fs::remove_dir_all(&failures);

    std::fs::create_dir_all(new_dir)?;
    std::fs::create_dir_all(changed_dir)?;
    std::fs::create_dir_all(failures)?;

    Ok(())
}

pub fn old_path(output: &Path) -> PathBuf {
    output.join("old")
}

pub fn new_path(output: &Path) -> PathBuf {
    output.join("new")
}

pub fn changed_path(output: &Path) -> PathBuf {
    output.join("changed")
}

pub fn failures_path(output: &Path) -> PathBuf {
    output.join("failures")
}

pub fn has_snapshot(rom_name: &str, snapshot_dir: &Path) -> Option<PathBuf> {
    let snapshot = snapshot_dir.join(format!("{rom_name}.png"));

    if snapshot.exists() {
        Some(snapshot)
    } else {
        None
    }
}

/// Setup the directory where one can save the Snapshots for tests.
///
/// A test with an associated snapshot will fail if it starts to differ from the established baseline.
pub fn setup_snapshot_directory(snapshot: &Path) -> anyhow::Result<()> {
    Ok(std::fs::create_dir_all(snapshot)?)
}

pub fn get_rom_fs_name(path: &Path) -> String {
    path.file_stem()
        .expect("Failed to get rom stem")
        .to_string_lossy()
        .to_string()
}
