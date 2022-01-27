mod bus;
pub mod cartridge;
mod cpu;
pub mod emulator;
mod ppu;
pub mod scheduler;
mod utils;

pub use ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

macro_rules! cpu_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "cpu-logging")]
        println!($($arg)*);
    }
}

use cpu_log;
