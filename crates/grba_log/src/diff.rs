use crate::format::DiffItem;
use crate::InstructionSnapshot;
use anyhow::Context;
use itertools::Itertools;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(clap::Args, Debug)]
pub struct DiffCommand {
    /// The path to the GRBA log file to parse
    #[clap(short, long, env, default_value = "./emu.logbin")]
    emu_log: PathBuf,
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

/// Handle the `Diff` command, used to find the first difference between the two provided logs.
pub fn handle_diff(cmd: DiffCommand) -> anyhow::Result<()> {
    let now = Instant::now();
    let emu_log = crate::open_mmap(&cmd.emu_log).context("Could not find emulator log, is the path correct?")?;
    let other_log = crate::open_mmap(&cmd.other_log).context("Could not find the other log, is the path correct?")?;

    let emu_contents = InstructionSnapshot::parse(&*emu_log).context("Failed to parse emu contents")?;
    let other_contents = InstructionSnapshot::parse(&*other_log).context("Failed to parse other contents")?;

    let result = emu_contents
        .iter()
        .zip(other_contents.iter())
        .find_position(|(emu, other)| emu != other)
        .map(|(idx, _)| idx);

    if let Some(idx) = result {
        // Display the 10 instructions around the contents.
        let range = (idx.saturating_sub(cmd.before)..=idx.saturating_add(cmd.after));
        let to_display_emu = &emu_contents[range.clone()];
        let to_display_other = &other_contents[range.clone()];

        let items: Vec<_> = range
            .zip(to_display_emu)
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

        println!("{}", table);

        println!("{} searching in {:.2?}", "Finished".bright_green(), now.elapsed());

        Err(anyhow::anyhow!(
            "{}: `{}`",
            "Difference found at index".bright_red(),
            idx.yellow()
        ))
    } else {
        println!("{} searching in {:.2?}", "Finished".bright_green(), now.elapsed());
        println!(
            "Searched: `{}` entries, no differences found!",
            other_contents.len().yellow()
        );

        Ok(())
    }
}
