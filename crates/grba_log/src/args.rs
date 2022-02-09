use clap::{AppSettings, Parser, Subcommand};
use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[clap(version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub commands: SubCommands,
}

#[derive(Subcommand, Debug)]
pub enum SubCommands {
    /// Diff two binary log files
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    #[clap(alias = "d")]
    Diff {
        /// The path to the GRBA log file to parse
        #[clap(short, long, env, default_value = "./emu.logbin")]
        emu_log: PathBuf,

        /// The path to the other emulator's log file.
        /// This will be used as the reference.
        #[clap(short, long, env, default_value = "./other.logbin")]
        other_log: PathBuf,
    },
    /// Will run the provided `rom_path` until the log and `other_log` diverge.
    /// Subsequently, the changes will be printed.
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    #[clap(alias = "r")]
    Run {
        /// The path to the GRBA log file to parse
        rom_path: PathBuf,

        /// The path to the other emulator's log file.
        /// This will be used as the reference.
        #[clap(short, long, env, default_value = "./other.bin")]
        other_log: PathBuf,
    },
}
