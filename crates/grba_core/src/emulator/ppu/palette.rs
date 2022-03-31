#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct RGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// 15 Bit BGR color
pub struct Palette {
    blue: u8,
    green: u8,
    red: u8,
}
