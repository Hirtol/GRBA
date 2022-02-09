use args::{Args, SubCommands};
use clap::Parser;
use format::InstructionSnapshot;
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use zerocopy::{ByteSlice, LayoutVerified};

mod args;
mod diff;
mod format;
mod run;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.commands {
        SubCommands::Diff { emu_log, other_log } => {
            diff::handle_diff(emu_log, other_log)?;
        }
        SubCommands::Run { rom_path, other_log } => {
            run::handle_run(rom_path, other_log)?;
        }
    }

    Ok(())
}

fn open_mmap(path: &Path) -> anyhow::Result<Mmap> {
    let file = File::open(path)?;

    unsafe { Ok(Mmap::map(&file)?) }
}
