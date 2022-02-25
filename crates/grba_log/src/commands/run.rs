use crate::bin_logger::InstructionLogger;
use crate::format::{DiffItem, DiffItemWithInstr};
use crate::InstructionSnapshot;
use anyhow::Context;
use grba_core::emulator::{EmuOptions, GBAEmulator};
use std::fs::read;
use std::panic;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(clap::Args, Debug)]
pub struct RunCommand {
    /// The path to the GRBA log file to parse
    rom_path: PathBuf,
    /// The path to the GBA Bios
    #[clap(long, env)]
    bios: Option<PathBuf>,
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
    /// Ignores the provided registers when comparing the logs
    #[clap(long)]
    ignore: Vec<usize>,
    /// The amount of times to ignore a difference before stopping the run command. Not affected by `ignore`
    #[clap(short, long, default_value = "0")]
    ignore_amount: u32,
}

/// Handle the `Run` command, where our emulator is ran until a difference is found.
/// Can give more useful information, as the entire state can be dumped.
pub fn handle_run(cmd: RunCommand) -> anyhow::Result<()> {
    let now = Instant::now();
    let logger = crate::bin_logger::setup_logger(cmd.before + 1);

    RunExecutor {
        cmd,
        start_time: now,
        logger,
    }
    .run()
}

pub struct RunExecutor {
    cmd: RunCommand,
    start_time: Instant,
    logger: &'static InstructionLogger,
}

impl RunExecutor {
    /// Execute the `Run` command.
    pub fn run(mut self) -> anyhow::Result<()> {
        let mut emulator = create_emulator(&self.cmd.rom_path, self.cmd.bios.as_deref())?;

        let other_log =
            crate::open_mmap(&self.cmd.other_log).context("Could not find the other log, is the path correct?")?;

        let other_contents = &InstructionSnapshot::parse(&*other_log).context("Failed to parse other contents")?[2..];

        for (idx, other_instr) in other_contents.iter().enumerate() {
            emulator.step_instruction();
            let current_frame = self.logger.get_most_recent();

            if other_instr != current_frame.registers.as_ref() {
                let differences = other_instr.get_differing_fields(current_frame.registers.as_ref());
                // Skip if the only differences are in ignored registers
                if differences.iter().all(|diff| self.cmd.ignore.contains(diff)) {
                    continue;
                }

                if self.cmd.ignore_amount > 0 {
                    self.cmd.ignore_amount -= 1;
                    continue;
                }

                self.handle_difference(&mut emulator, other_contents, idx)?;
            }
        }

        crate::commands::show_success(self.start_time, other_contents.len());

        Ok(())
    }

    /// Handle a confirmed difference in the logs.
    fn handle_difference(
        &mut self,
        emulator: &mut GBAEmulator,
        other_contents: &[InstructionSnapshot],
        idx: usize,
    ) -> anyhow::Result<()> {
        let mut before = self.logger.history.lock().unwrap().clone();

        for _ in 0..self.cmd.after {
            emulator.step_instruction();
            before.push(self.logger.get_most_recent());
        }

        let range = idx.saturating_sub(self.cmd.before)..=idx.saturating_add(self.cmd.after);
        let to_display_other = &other_contents[range.clone()];
        let items: Vec<_> = range
            .zip(&before)
            .zip(to_display_other)
            .map(|((i, emu), other)| DiffItemWithInstr {
                instr: emu.instruction,
                diff_item: DiffItem {
                    instr_idx: i,
                    emu_instr: emu.registers.as_ref(),
                    other_instr: other,
                    is_error: idx == i,
                    different_fields: other.get_differing_fields(emu.registers.as_ref()),
                },
            })
            .collect();

        let table = tabled::Table::new(items)
            .with(tabled::Style::PSEUDO)
            .with(tabled::Modify::new(tabled::Column(2..=2)).with(tabled::Alignment::left()));

        anyhow::bail!(crate::commands::show_diff_found(self.start_time, idx, table))
    }
}

fn create_emulator(rom: &Path, bios: Option<&Path>) -> anyhow::Result<GBAEmulator> {
    let rom_data = std::fs::read(rom)?;
    let ram_data = vec![0u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE];
    let cartridge = grba_core::emulator::cartridge::Cartridge::new(rom_data, Box::new(ram_data));

    let mut emu_opts = EmuOptions::default();
    if let Some(bios) = bios {
        let bios_data = std::fs::read(bios)?;
        emu_opts.bios = Some(bios_data);
    }

    Ok(grba_core::emulator::GBAEmulator::new(cartridge, emu_opts))
}
