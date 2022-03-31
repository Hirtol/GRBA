use crate::emulator::ppu::RGBA;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct RgbaFrame(pub Box<[RGBA; crate::FRAMEBUFFER_SIZE]>);

impl Default for RgbaFrame {
    fn default() -> Self {
        Self(crate::box_array![RGBA::default(); crate::FRAMEBUFFER_SIZE])
    }
}

impl Deref for RgbaFrame {
    type Target = [RGBA; crate::FRAMEBUFFER_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RgbaFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RgbaFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// Transform the internal buffer into a slice and return it.
    ///
    /// # Safety
    ///
    /// So long as [RGBA] remains `#[repr(C)]` it's safe.
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: Since RGBA is #[repr(C)] and the array is boxed, we can safely create a bounded slice reference to it.
        unsafe {
            std::slice::from_raw_parts(
                self.0.as_ptr() as *const u8,
                crate::FRAMEBUFFER_SIZE * core::mem::size_of::<RGBA>(),
            )
        }
    }
}
