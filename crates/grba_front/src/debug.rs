use grba_core::logging::BinaryLogger;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct DebugLogger {
    writer: BufWriter<File>,
}

impl DebugLogger {
    pub fn new(log_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(DebugLogger {
            writer: BufWriter::new(File::create(log_path)?),
        })
    }
}

impl BinaryLogger for DebugLogger {
    fn log_binary(&mut self, data: &[u8]) {
        self.writer.write_all(data).unwrap();
    }
}
