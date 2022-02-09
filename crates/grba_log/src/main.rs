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

// use miette::Diagnostic;
// use miette::{Result, SourceSpan};
// use thiserror::Error;
//
// fn main() -> Result<()> {
//     Err(MyErrorType {
//         src: "Demo\n yooo \n too".to_string(),
//         err_span: (2, 3).into(),
//         snip2: (1, 2),
//     })?;
//
//     return Ok(());
// }
//
// #[derive(Diagnostic, Debug, Error)]
// #[error("oops")]
// #[diagnostic(code(my_lib::random_error))]
// pub struct MyErrorType {
//     // The `Source` that miette will use.
//     #[source_code]
//     src: String,
//
//     // This will underline/mark the specific code inside the larger
//     // snippet context.
//     #[label = "This is the highlight"]
//     err_span: SourceSpan,
//
//     // You can add as many labels as you want.
//     // They'll be rendered sequentially.
//     #[label("This is bad")]
//     snip2: (usize, usize), // (usize, usize) is Into<SourceSpan>!
// }
