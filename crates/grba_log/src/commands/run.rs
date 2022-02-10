use crate::format::DiffItem;
use crate::InstructionSnapshot;
use anyhow::Context;
use grba_core::emulator::{EmuOptions, GBAEmulator};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

#[derive(clap::Args, Debug)]
pub struct RunCommand {
    /// The path to the GRBA log file to parse
    rom_path: PathBuf,
    /// The path to the other emulator's log file.
    /// This will be used as the reference.
    #[clap(short, long, env, default_value = "./other.logbin")]
    other_log: PathBuf,
    /// The amount of entries to display *before* a discovered difference in the logs
    #[clap(short, long, default_value = "3")]
    before: usize,
    /// The amount of entries to display *after* a discovered difference in the logs
    #[clap(short, long, default_value = "3")]
    after: usize,
}

/// Handle the `Run` command, where our emulator is ran until a difference is found.
/// Can give more useful information, as the entire state can be dumped.
pub fn handle_run(cmd: RunCommand) -> anyhow::Result<()> {
    let now = Instant::now();
    let logger = setup_logger(cmd.before + 1);
    let mut emulator = create_emulator(&cmd.rom_path)?;

    let other_log = crate::open_mmap(&cmd.other_log).context("Could not find the other log, is the path correct?")?;

    let other_contents = &InstructionSnapshot::parse(&*other_log).context("Failed to parse other contents")?[2..];

    for idx in 0..other_contents.len() {
        emulator.step_instruction();
        let other_instr = &other_contents[idx];
        let current_instr = logger.get_most_recent();

        if other_instr != &current_instr {
            let mut before = logger.history.lock().unwrap().clone();

            for _ in 0..cmd.after {
                emulator.step_instruction();
                before.push(logger.get_most_recent());
            }

            let range = (idx.saturating_sub(cmd.before)..=idx.saturating_add(cmd.after));
            let to_display_other = &other_contents[range.clone()];
            let items: Vec<_> = range
                .zip(&before)
                .zip(to_display_other)
                .map(|((i, emu), other)| DiffItem {
                    instr_idx: i,
                    emu_instr: emu,
                    other_instr: other,
                    is_error: idx == i,
                    different_fields: emu.get_differing_fields(other),
                })
                .collect();

            let table = tabled::Table::new(items).with(tabled::Style::PSEUDO);

            return Err(anyhow::anyhow!(crate::commands::show_diff_found(now, idx, table)));
        }
    }

    crate::commands::show_success(now, other_contents.len());

    Ok(())
}

fn setup_logger(before: usize) -> &'static InstructionLogger {
    // Since this is the only command we'll execute we're just gonna leak the logger.
    let logger = Box::leak(Box::new(InstructionLogger::new(before)));
    grba_core::logging::set_logger(logger);
    logger
}

fn create_emulator(rom: &Path) -> anyhow::Result<GBAEmulator> {
    let rom_data = std::fs::read(rom)?;
    let ram_data = vec![0u8; grba_core::cartridge::CARTRIDGE_RAM_SIZE];
    let cartridge = grba_core::cartridge::Cartridge::new(rom_data, Box::new(ram_data));
    Ok(grba_core::emulator::GBAEmulator::new(cartridge, EmuOptions::default()))
}

#[derive(Default)]
pub struct InstructionLogger {
    history: Mutex<Vec<InstructionSnapshot>>,
}

impl InstructionLogger {
    pub fn new(history_size: usize) -> InstructionLogger {
        InstructionLogger {
            history: Mutex::new(Vec::with_capacity(history_size)),
        }
    }

    pub fn get_most_recent(&self) -> InstructionSnapshot {
        let lock = self.history.lock().unwrap();
        lock.last().unwrap().clone()
    }
}

impl grba_core::logging::BinaryLogger for InstructionLogger {
    fn log_binary(&self, target: &str, data: &[u8]) {
        if target == grba_core::logging::BIN_TARGET_REGISTER {
            let instructions = InstructionSnapshot::parse(data).unwrap();
            let mut lock = self.history.lock().unwrap();

            if lock.len() == lock.capacity() {
                lock.rotate_left(1);
                *lock.last_mut().unwrap() = instructions[0].clone();
            } else {
                lock.push(instructions[0].clone());
            }
        }
    }
}
