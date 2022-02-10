use grba_core::logging::InstructionFrame;
use std::sync::Mutex;

pub fn setup_logger(before: usize) -> &'static InstructionLogger {
    // Since this is the only command we'll execute we're just gonna leak the logger.
    let logger = Box::leak(Box::new(InstructionLogger::new(before)));
    grba_core::logging::set_logger(logger);
    logger
}

#[derive(Default)]
pub struct InstructionLogger {
    pub history: Mutex<Vec<InstructionFrame>>,
}

impl InstructionLogger {
    pub fn new(history_size: usize) -> InstructionLogger {
        InstructionLogger {
            history: Mutex::new(Vec::with_capacity(history_size)),
        }
    }

    pub fn get_most_recent(&self) -> InstructionFrame {
        let lock = self.history.lock().unwrap();
        lock.last().unwrap().clone()
    }
}

impl grba_core::logging::BinaryLogger for InstructionLogger {
    fn log_binary(&self, target: &str, data: &[u8]) {
        if target == grba_core::logging::BIN_TARGET_FRAME {
            let frame = InstructionFrame::from_bytes(data);
            let mut lock = self.history.lock().unwrap();

            if lock.len() == lock.capacity() {
                lock.rotate_left(1);
                *lock.last_mut().unwrap() = frame.clone();
            } else {
                lock.push(frame.clone());
            }
        }
    }
}
