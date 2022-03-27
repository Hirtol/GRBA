pub mod emulator;
mod joypad;
pub mod logging;
pub mod scheduler;
mod utils;

pub use emulator::ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
pub use joypad::InputKeys;
/// The total framebuffer size that would be returned each frame.
/// Format is [crate::emulator::ppu::RGBA].
pub const FRAMEBUFFER_SIZE: usize = (DISPLAY_WIDTH * DISPLAY_HEIGHT) as usize;
/// The amount of frames per second a normal GBA would display.
pub const REFRESH_RATE: f32 = 59.7275;
/// The clock speed of the ARM7TDMI CPU in Hz.
pub const CLOCK_SPEED: u32 = 16_777_216;

pub const CLOCKS_PER_FRAME: u32 = (CLOCK_SPEED as f32 / REFRESH_RATE) as u32;
