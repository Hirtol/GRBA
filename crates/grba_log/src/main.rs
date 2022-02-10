use args::{Args, SubCommands};
use clap::Parser;
use commands::{diff, run};
use format::InstructionSnapshot;
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use zerocopy::{AsBytes, ByteSlice, LayoutVerified};

mod args;
mod commands;
mod format;
mod bin_logger;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.commands {
        SubCommands::Diff(cmd) => {
            diff::handle_diff(cmd)?;
        }
        SubCommands::Run(cmd) => {
            run::handle_run(cmd)?;
        }
    }

    Ok(())
}

fn open_mmap(path: &Path) -> anyhow::Result<Mmap> {
    let file = File::open(path)?;

    unsafe { Ok(Mmap::map(&file)?) }
}
