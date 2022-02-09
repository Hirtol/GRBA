use crate::InstructionSnapshot;
use anyhow::Context;
use itertools::Itertools;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::time::Instant;
use tabled::*;

/// Handle the `Diff` command, used to find the first difference between the two provided logs.
pub fn handle_diff(emu_log: PathBuf, other_log: PathBuf) -> anyhow::Result<()> {
    let now = Instant::now();
    let emu_log = crate::open_mmap(&emu_log).context("Could not find emulator log, is the path correct?")?;
    let other_log = crate::open_mmap(&other_log).context("Could not find the other log, is the path correct?")?;

    let emu_contents = InstructionSnapshot::parse(&*emu_log).context("Failed to parse emu contents")?;
    let other_contents = InstructionSnapshot::parse(&*other_log).context("Failed to parse other contents")?;

    let result = emu_contents
        .iter()
        .zip(other_contents.iter())
        .find_position(|(emu, other)| emu != other)
        .map(|(idx, _)| idx);

    println!("{} searching in {:.2?}", "Finished".bright_green(), now.elapsed());

    if let Some(idx) = result {
        // Display the 10 instructions around the contents.
        let range = (idx.saturating_sub(5)..idx.saturating_add(5));
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

        Err(anyhow::anyhow!(
            "{}: `{}`",
            "Difference found at index".bright_red(),
            idx.yellow()
        ))
    } else {
        println!(
            "Searched: `{}` registries, no differences found!",
            emu_contents.len().yellow()
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct DiffItem<'a> {
    /// The index of the executed instruction in the log
    instr_idx: usize,
    /// Whether this was the first difference causing an error
    is_error: bool,
    /// The indexes of the fields from [InstructionSnapshot]s which are different
    different_fields: Vec<usize>,
    /// The [InstructionSnapshot] from the emulator log
    emu_instr: &'a InstructionSnapshot,
    /// The [InstructionSnapshot] from the other emulator log
    other_instr: &'a InstructionSnapshot,
}

impl<'a> Tabled for DiffItem<'a> {
    const LENGTH: usize = 2;

    fn fields(&self) -> Vec<String> {
        {
            let mut out = Vec::with_capacity(Self::LENGTH);

            if self.is_error {
                out.push(format!(
                    "{}\n{}",
                    self.instr_idx.bright_magenta(),
                    "(X)".bright_magenta()
                ));
            } else {
                out.push(format!("{}", self.instr_idx));
            }

            let name_table = builder::Builder::new()
                .set_header(["Emulator"])
                .add_row(["Emu"])
                .add_row(["Other"])
                .build();

            let mut register_table = tabled::Table::new([self.emu_instr, self.other_instr]);

            for &column_idx in &self.different_fields {
                register_table = register_table
                    .with(Modify::new(Column(column_idx..=column_idx)).with(Format(|s| s.bright_red().to_string())));
            }

            let table = name_table
                .with(Concat::horizontal(register_table))
                .with(Style::PSEUDO_CLEAN);

            out.push(format!("{}", table));

            out
        }
    }

    fn headers() -> Vec<String> {
        vec!["Index".to_string(), "Registers".to_string()]
    }
}
