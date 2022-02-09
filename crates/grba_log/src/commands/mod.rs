use owo_colors::OwoColorize;
use std::time::Instant;
use tabled::Table;

pub mod diff;
pub mod run;

fn show_success(now: Instant, total_instr_count: usize) {
    println!("{} searching in {:.2?}", "Finished".bright_green(), now.elapsed());
    println!(
        "Searched: `{}` entries, no differences found!",
        total_instr_count.yellow()
    );
}

fn show_diff_found(now: Instant, idx: usize, table: Table) -> String {
    println!("{}", table);

    println!("{} searching in {:.2?}", "Finished".bright_green(), now.elapsed());

    format!("{}: `{}`", "Difference found at index".bright_red(), idx.yellow())
}
