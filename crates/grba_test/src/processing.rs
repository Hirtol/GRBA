use std::path::{Path, PathBuf};
use std::time::Duration;

use image::{EncodableLayout, ImageBuffer};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{setup, EmuContext, RunnerError, RunnerOutput};

pub type TestOutput = EmuContext<TestOutputContext>;

#[derive(Debug)]
pub struct TestOutputContext {
    pub time_taken: Option<Duration>,
    pub output: TestOutputType,
}

#[derive(Debug)]
pub enum TestOutputType {
    Unchanged,
    Changed {
        changed_path_dump: PathBuf,
        old_path: PathBuf,
    },
    Failure {
        failure_path: PathBuf,
        snapshot_path: PathBuf,
    },
    Passed,
    Error {
        reason: anyhow::Error,
    },
}

impl TestOutputType {
    pub fn to_error(&self) -> Option<&anyhow::Error> {
        match self {
            TestOutputType::Error { reason } => Some(reason),
            _ => None,
        }
    }
}

pub fn process_results(
    results: Vec<Result<RunnerOutput, RunnerError>>,
    output: &Path,
    snapshot_dir: &Path,
) -> Vec<TestOutput> {
    results
        .into_par_iter()
        .map(|runner_output| {
            let runner_output = match runner_output {
                Ok(output) => output,
                Err(e) => return e.into(),
            };
            let lambda = || {
                let image_frame: ImageBuffer<image::Rgba<u8>, &[u8]> = if let Some(img) = image::ImageBuffer::from_raw(
                    grba_core::DISPLAY_WIDTH,
                    grba_core::DISPLAY_HEIGHT,
                    runner_output.context.frame_output.as_bytes(),
                ) {
                    img
                } else {
                    anyhow::bail!("Failed to turn GRBA framebuffer to dynamic image")
                };

                let result_name = format!("{}.png", &runner_output.rom_name);
                let new_path = setup::new_path(output).join(&result_name);

                image_frame.save(&new_path)?;

                let output = if let Some(snapshot) = setup::has_snapshot(&runner_output.rom_name, snapshot_dir) {
                    // Time to see if our snapshot is still correct
                    let snapshot_data = image::open(&snapshot)?;

                    if snapshot_data.as_bytes() != image_frame.as_bytes() {
                        let failure_path = setup::failures_path(output).join(&result_name);
                        std::fs::copy(&new_path, &failure_path)?;

                        TestOutputType::Failure {
                            failure_path,
                            snapshot_path: snapshot,
                        }
                    } else {
                        TestOutputType::Passed
                    }
                } else {
                    // Just check if there has been *any* change at all
                    let old_path = setup::old_path(output).join(&result_name);

                    if old_path.exists() {
                        let old_data = image::open(&old_path)?;

                        if old_data.as_bytes() != image_frame.as_bytes() {
                            let changed_path = setup::changed_path(output).join(&result_name);
                            std::fs::copy(&new_path, &changed_path)?;

                            TestOutputType::Changed {
                                changed_path_dump: changed_path,
                                old_path,
                            }
                        } else {
                            TestOutputType::Unchanged
                        }
                    } else {
                        TestOutputType::Unchanged
                    }
                };

                Ok(output)
            };

            match lambda() {
                Ok(output) => runner_output.map(|context| TestOutputContext {
                    time_taken: Some(context.time_taken),
                    output,
                }),
                Err(e) => runner_output.map(|context| TestOutputContext {
                    time_taken: Some(context.time_taken),
                    output: TestOutputType::Error { reason: e },
                }),
            }
        })
        .collect()
}

impl From<RunnerError> for TestOutput {
    fn from(value: RunnerError) -> Self {
        value.map(|error| TestOutputContext {
            time_taken: None,
            output: TestOutputType::Error { reason: error },
        })
    }
}
