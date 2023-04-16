use anyhow::Context;
use clap::Parser;
use emu_test_runner::formatters::simple::SimpleConsoleFormatter;
use emu_test_runner::options::EmuRunnerOptions;
use emu_test_runner::outputs::FrameOutput;
use emu_test_runner::EmuTestRunner;

use crate::config::{ClapArgs, TestSequenceInstructions};
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

    let (tests, test_id_sequence_map) = utils::find_all_tests(&test_roms, &config)?;

    let formatter = Box::new(SimpleConsoleFormatter::new().with_progress(tests.len() as u64));
    let options = EmuRunnerOptions {
        output_path,
        snapshot_path: snapshots,
        num_threads: clap_args.num_threads.unwrap_or(config.num_threads),
        expected_frame_width: grba_core::DISPLAY_WIDTH as usize,
        expected_frame_height: grba_core::DISPLAY_HEIGHT as usize,
        put_sequence_tests_in_subfolder: true,
        copy_comparison_image: true,
    };
    let runner = EmuTestRunner::new(formatter, options)?;

    let bios = std::fs::read(bios_path)?;

    runner.run_tests(tests.into_iter(), |test, rom_data| {
        if let Some(custom_conf) = test_id_sequence_map.get(&test.rom_id) {
            let frames_to_run = custom_conf.num_frames;

            if let Some(sequence) = custom_conf.sequence {
                run_sequence_test(sequence, rom_data, frames_to_run, &bios)
            } else {
                vec![run_normal_test(rom_data, frames_to_run, &bios)]
            }
        } else {
            vec![run_normal_test(rom_data, clap_args.frames, &bios)]
        }
    })?;

    Ok(())
}

pub fn run_normal_test(rom: Vec<u8>, frames_to_run: u32, bios: &[u8]) -> FrameOutput {
    let mut emu = construct_emu(rom, bios);

    for _ in 0..frames_to_run {
        emu.run_to_vblank();
    }

    capture_emulator_frame(None, &mut emu)
}

pub fn run_sequence_test(
    sequence: &[TestSequenceInstructions],
    rom: Vec<u8>,
    frames_to_run: u32,
    bios: &[u8],
) -> Vec<FrameOutput> {
    let mut emu = construct_emu(rom, bios);
    let mut output_frames = Vec::with_capacity(sequence.len());

    for _ in 0..frames_to_run {
        emu.run_to_vblank();
    }

    output_frames.push(capture_emulator_frame(None, &mut emu));

    for instruction in sequence {
        handle_instruction(instruction, &mut emu, &mut output_frames);
    }

    output_frames
}

fn handle_instruction(
    instruction: &TestSequenceInstructions,
    emu: &mut grba_core::emulator::GBAEmulator,
    frame_buffer: &mut Vec<FrameOutput>,
) {
    match instruction {
        TestSequenceInstructions::DumpFrame(name) => {
            frame_buffer.push(capture_emulator_frame(Some(name.clone()), emu));
        }
        TestSequenceInstructions::AdvanceFrames(to_advance) => {
            for _ in 0..*to_advance {
                emu.run_to_vblank();
            }
        }
        TestSequenceInstructions::Input(key) => {
            handle_instruction(&TestSequenceInstructions::HoldInputFor(*key, 1), emu, frame_buffer)
        }
        TestSequenceInstructions::HoldInputFor(key, to_advance) => {
            emu.key_down(*key);
            handle_instruction(&TestSequenceInstructions::AdvanceFrames(*to_advance), emu, frame_buffer);
            emu.key_up(*key);
            emu.run_to_vblank();
        }
    }
}

fn construct_emu(rom: Vec<u8>, bios: &[u8]) -> grba_core::emulator::GBAEmulator {
    let emu_options = grba_core::emulator::EmuOptions {
        skip_bios: true,
        bios: Some(bios.to_owned()),
        debugging: false,
    };
    let cartridge = grba_core::emulator::cartridge::Cartridge::new(rom, Box::new(MemoryRam::default()));
    let emu = grba_core::emulator::GBAEmulator::new(cartridge, emu_options);

    emu
}

fn capture_emulator_frame(suffix: Option<String>, emu: &mut grba_core::emulator::GBAEmulator) -> FrameOutput {
    FrameOutput {
        tag: suffix,
        frame: emu_test_runner::outputs::RgbaFrame(emu.frame_buffer().as_bytes().to_vec()),
    }
}
