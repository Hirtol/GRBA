mod bus;
pub mod cartridge;
mod cpu;
pub mod emulator;
mod ppu;
pub mod scheduler;
mod utils;

pub use ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
/// The total framebuffer size that would be returned each frame.
/// Format is `RGBA_u8`.
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 4) as usize;

macro_rules! cpu_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "cpu-logging")]
        println!($($arg)*);
    }
}

use cpu_log;
