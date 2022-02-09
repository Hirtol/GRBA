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
    Diff(crate::diff::DiffCommand),
    /// Will run the provided `rom_path` until the log and `other_log` diverge.
    /// Subsequently, the changes will be printed.
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    #[clap(alias = "r")]
    Run(crate::run::RunCommand),
}
