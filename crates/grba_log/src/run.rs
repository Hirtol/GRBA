use std::path::PathBuf;

/// Handle the `Run` command, where our emulator is ran until a difference is found.
/// Can give more useful information, as the entire state can be dumped.
pub fn handle_run(rom: PathBuf, other_log: PathBuf) -> anyhow::Result<()> {
    Ok(())
}
