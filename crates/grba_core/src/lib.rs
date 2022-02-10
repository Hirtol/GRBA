pub mod emulator;
mod joypad;
pub mod logging;
pub mod scheduler;
mod utils;

pub use emulator::ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
pub use joypad::InputKeys;
/// The total framebuffer size that would be returned each frame.
/// Format is `RGBA_u8`.
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT * 4) as usize;
/// The amount of frames per second a normal GBA would display.
pub const REFRESH_RATE: f32 = 59.7275;
/// The clock speed of the ARM7TDMI CPU in Hz.
pub const CLOCK_SPEED: u32 = 16_777_216;

pub const CLOCKS_PER_FRAME: u32 = (CLOCK_SPEED as f32 / REFRESH_RATE) as u32;

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
