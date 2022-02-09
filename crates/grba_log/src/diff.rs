use crate::InstructionSnapshot;
use anyhow::Context;
use std::path::PathBuf;

/// Handle the `Diff` command, used to find the first difference between the two provided logs.
pub fn handle_diff(emu_log: PathBuf, other_log: PathBuf) -> anyhow::Result<()> {
    let emu_log = crate::open_mmap(&emu_log).context("Could not find emulator log, is the path correct?")?;
    let other_log = crate::open_mmap(&other_log).context("Could not find the other log, is the path correct?")?;

    let emu_contents = InstructionSnapshot::parse(&*emu_log).context("Failed to parse emu contents")?;
    let contents = InstructionSnapshot::parse(&*other_log).context("Failed to parse other contents")?;

    emu_contents
        .iter()
        .zip(contents.iter())
        .enumerate()
        .try_for_each(|(index, (emu, other))| {
            if emu != other {
                println!("Emulator: {:#?}", emu);
                println!("Other In:{:#?}", other);
                anyhow::bail!("Difference found at index: `{}`", index);
            }

            Ok(())
        })?;

    println!("No differences found, all good!");

    Ok(())
}
