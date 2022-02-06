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
/// The clock speed of the ARM7TDMI CPU.
pub const CLOCK_SPEED: u32 = 16_780_000;

pub const CLOCKS_PER_FRAME: u32 = (CLOCK_SPEED as f32 / REFRESH_RATE) as u32;

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

/// Check whether the provided bit is set, returns a `bool`
///
/// ```rust
/// # use grba_core::check_bit;
/// let bit_is_set = check_bit!(0b0001_0000, 4);
///
/// assert!(bit_is_set);
/// ```
#[macro_export]
macro_rules! check_bit {
    ($val:expr, $bit:expr) => {
        ($val & (1 << $bit)) != 0
    };
}

/// Return the bits in the specified range.
/// Will be optimised by the compiler to a simple `shift` and `and`.
///
/// ```rust
/// # use grba_core::get_bits;
/// // Get bits in the range of 12..=15
/// let value = get_bits!(0xBEEF, 12, 15);
///
/// assert_eq!(value, 0xB);
/// ```
#[macro_export]
macro_rules! get_bits {
    ($val:expr, $start:expr, $end_inclusive:expr) => {
        ($val >> $start) & ((1 << ($end_inclusive - $start + 1)) - 1)
    };
}

use cpu_log;
