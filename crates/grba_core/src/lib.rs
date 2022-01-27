mod bus;
mod cartridge;
mod cpu;
pub mod emulator;
mod ppu;
pub mod scheduler;
mod utils;

macro_rules! cpu_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "cpu-logging")]
        println!($($arg)*);
    }
}

use cpu_log;
