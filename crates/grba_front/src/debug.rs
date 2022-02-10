//! This is purely used to efficiently log the state of the emulator after every instruction
//! This is why this uses a lot of dangerous unsafe (aka, it will go wrong in any scenario other than the current one!)
use grba_core::logging::BinaryLogger;
use std::cell::UnsafeCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Create a [DebugLogger] with the output being written to the given `log_path`.
pub fn setup_emulator_logger(log_path: impl AsRef<Path>) -> anyhow::Result<&'static DebugLogger> {
    let logger = DebugLogger::new(log_path)?;
    let leaked_logger = Box::leak(Box::new(logger));
    grba_core::logging::set_logger(leaked_logger);
    Ok(leaked_logger)
}

pub struct DebugLogger {
    writer: UnsafeCell<BufWriter<File>>,
}

impl DebugLogger {
    pub fn new(log_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(DebugLogger {
            writer: BufWriter::new(File::create(log_path)?).into(),
        })
    }
}

unsafe impl Sync for DebugLogger {}

impl BinaryLogger for DebugLogger {
    fn log_binary(&self, target: &str, data: &[u8]) {
        if target == grba_core::logging::BIN_TARGET_FRAME {
            // There is no safety
            unsafe {
                let frame = grba_core::logging::InstructionFrame::from_bytes(data);
                let writer = &mut *self.writer.get();
                writer.write_all(frame.registers.as_ref()).unwrap();
            }
        }
    }
}
