use std::path::{Path, PathBuf};

use anyhow::Context;
use owo_colors::{CssColors, OwoColorize};

use crate::processing::{TestOutput, TestOutputType};

pub struct SimpleReporter {
    output_path: PathBuf,
}

impl SimpleReporter {
    pub fn new(output_path: impl Into<PathBuf>) -> Self {
        Self {
            output_path: output_path.into(),
        }
    }
}

impl SimpleReporter {
    pub fn handle_start(&self, test_roms: &[impl AsRef<Path>]) {
        println!("=== Running {} Snapshot Tests ===\n", test_roms.len().green())
    }

    pub fn handle_complete_tests(&self, reports: &[TestOutput]) {
        let (mut passed, mut failed, mut unchanged, mut changed, mut errors) = (vec![], vec![], vec![], vec![], vec![]);

        for report in reports {
            match report.context.output {
                TestOutputType::Unchanged => unchanged.push(report),
                TestOutputType::Changed { .. } => changed.push(report),
                TestOutputType::Failure { .. } => failed.push(report),
                TestOutputType::Passed => passed.push(report),
                TestOutputType::Error { .. } => errors.push(report),
            }
        }

        if !errors.is_empty() {
            println!("{}", "== Found errors ==".on_red());

            for error in &errors {
                println!("= {}({:?}) =", error.rom_name.red(), error.rom_path);
                println!(
                    "Error: {:#?}",
                    error.context.output.to_error().expect("Error wasn't present?")
                );
                println!()
            }

            println!()
        }

        if !failed.is_empty() {
            println!("{}\n", "== Found failures ==".on_color(CssColors::Orange));

            for fail in &failed {
                let (failure_path, snapshot_path) = match &fail.context.output {
                    TestOutputType::Failure {
                        failure_path,
                        snapshot_path,
                    } => (failure_path, snapshot_path),
                    _ => panic!(),
                };

                println!(
                    "= {}(file://{:?}) =",
                    fail.rom_name.color(CssColors::Orange),
                    fail.rom_path
                );
                println!("Failed snapshot test",);
                println!("Was: {:?}", failure_path);
                println!("Expected: {:?}", snapshot_path);
                println!()
            }

            println!()
        }

        if !changed.is_empty() {
            println!("{}\n", "== Found Changes ==".on_color(CssColors::RebeccaPurple));

            for change in &changed {
                let (changed_path_dump, old_path) = match &change.context.output {
                    TestOutputType::Changed {
                        changed_path_dump,
                        old_path,
                    } => (changed_path_dump, old_path),
                    _ => panic!(),
                };

                println!(
                    "- {}({:?})",
                    change.rom_name.color(CssColors::RebeccaPurple),
                    change.rom_path
                );
                println!("-- {changed_path_dump:?}");
            }

            println!()
        }

        let changed_len = changed.len();
        let failed_len = failed.len();
        let errors_len = errors.len();

        // Final Report
        println!("=== {} - Ran {} Tests ===", "Report".green(), reports.len().green());

        let no_longer_failing = passed
            .iter()
            .flat_map(|p| OutputDestinations::Old.compare(&p.rom_name, &self.output_path, OutputDestinations::New))
            .filter(|equal| !*equal)
            .count();
        if no_longer_failing > 0 {
            println!(
                "{: <16} {} ({} no longer failing)",
                "✔ Passed:",
                passed.len().green(),
                no_longer_failing.bright_green()
            );
        } else {
            println!("{: <16} {}", "✔ Passed:", passed.len().green());
        }

        println!("{: <15} {}", "😴 Same:", unchanged.len().green());
        println!(
            "{: <15} {}",
            "🔀 Changed:",
            if changed.is_empty() { 0.color(CssColors::Gray) } else { changed_len.color(CssColors::RebeccaPurple) }
        );

        println!(
            "{: <15} {}",
            "❌ Failed:",
            if failed.is_empty() { 0.color(CssColors::Gray) } else { failed_len.color(CssColors::Red) }
        );
        println!(
            "{: <15} {}",
            "💀 Died:",
            if errors.is_empty() { 0.color(CssColors::Gray) } else { errors_len.color(CssColors::Red) }
        );
    }
}

#[derive(Debug, Clone)]
pub enum OutputDestinations {
    Old,
    New,
    Failures,
    Changed,
    InMemory(Vec<u8>),
}

impl OutputDestinations {
    pub fn compare(&self, rom_name: &str, output_path: &Path, compare_to: OutputDestinations) -> anyhow::Result<bool> {
        let data = self.to_data(rom_name, output_path)?;
        let other_data = compare_to.to_data(rom_name, output_path)?;

        Ok(data == other_data)
    }

    pub fn to_path(&self, output_path: &Path) -> Option<PathBuf> {
        match self {
            OutputDestinations::Old => crate::setup::old_path(output_path).into(),
            OutputDestinations::New => crate::setup::new_path(output_path).into(),
            OutputDestinations::Failures => crate::setup::failures_path(output_path).into(),
            OutputDestinations::Changed => crate::setup::changed_path(output_path).into(),
            OutputDestinations::InMemory(_) => None,
        }
    }

    pub fn to_data(&self, rom_name: &str, output_path: &Path) -> anyhow::Result<Vec<u8>> {
        if let OutputDestinations::InMemory(data) = self {
            Ok(data.clone())
        } else {
            let picture_name = format!("{}.png", rom_name);
            let path = self
                .to_path(output_path)
                .context("Failed to get path")?
                .join(picture_name);

            Ok(std::fs::read(path)?)
        }
    }
}
