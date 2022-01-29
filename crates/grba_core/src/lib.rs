mod bus;
pub mod cartridge;
mod cpu;
pub mod emulator;
mod joypad;
mod ppu;
pub mod scheduler;
mod utils;

pub use joypad::InputKeys;
pub use ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
/// The total framebuffer size that would be returned each frame.
/// Format is `RGBA_u8`.
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 4) as usize;
/// The amount of frames per second a normal GBA would display.
pub const REFRESH_RATE: f32 = 59.7275;

macro_rules! cpu_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "cpu-logging")]
        println!($($arg)*);
    }
}

/// A macro similar to `vec![$elem; $size]` which returns a boxed array.
///
/// ```rustc
///     let _: Box<[u8; 1024]> = box_array![0; 1024];
/// ```
#[macro_export]
macro_rules! box_array {
    ($val:expr ; $len:expr) => {{
        // Use a generic function so that the pointer cast remains type-safe
        fn vec_to_boxed_array<T>(vec: Vec<T>) -> Box<[T; $len]> {
            let boxed_slice = vec.into_boxed_slice();

            let ptr = ::std::boxed::Box::into_raw(boxed_slice) as *mut [T; $len];

            unsafe { Box::from_raw(ptr) }
        }

        vec_to_boxed_array(vec![$val; $len])
    }};
}

use cpu_log;
