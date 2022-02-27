use crate::emulator::MemoryAddress;
use modular_bitfield_msb::prelude::{B19, B2, B27, B3, B4, B5, B7, B9};
use modular_bitfield_msb::{bitfield, BitfieldSpecifier};

pub const LCD_CONTROL_START: MemoryAddress = 0x0400_0000;
pub const LCD_CONTROL_END: MemoryAddress = 0x0400_0001;
/// Note, we don't bother with emulating this register as nothing uses it.
pub const GREEN_SWAP_START: MemoryAddress = 0x0400_0002;
pub const GREEN_SWAP_END: MemoryAddress = 0x0400_0003;
pub const LCD_STATUS_START: MemoryAddress = 0x0400_0004;
pub const LCD_STATUS_END: MemoryAddress = 0x0400_0005;
pub const LCD_VERTICAL_COUNTER_START: MemoryAddress = 0x0400_0006;
pub const LCD_VERTICAL_COUNTER_END: MemoryAddress = 0x0400_0007;

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct LcdControl {
    /// OBJ Window Display Flag (0=Off, 1=On) (Bit 15)
    obj_window_display: bool,
    /// Window 1 Display Flag   (0=Off, 1=On)
    window_1_display_flag: bool,
    /// Window 0 Display Flag   (0=Off, 1=On)
    window_0_display_flag: bool,
    /// Screen Display OBJ  (0=Off, 1=On)
    screen_display_obj: bool,
    /// Screen Display BG3  (0=Off, 1=On)
    screen_display_bg3: bool,
    /// Screen Display BG2  (0=Off, 1=On)
    screen_display_bg2: bool,
    /// Screen Display BG1  (0=Off, 1=On)
    screen_display_bg1: bool,
    /// Screen Display BG0  (0=Off, 1=On)
    screen_display_bg0: bool,
    /// Forced blank (1=Allow FAST access to VRAM,Palette,OAM)
    forced_blank: bool,
    /// OBJ Character VRAM Mapping (0=Two dimensional, 1=One dimensional)
    obj_character_vram_mapping: bool,
    /// H-Blank Interval Free  (1=Allow access to OAM during H-Blank)
    h_blank_interval_free: bool,
    /// Display Frame Select   (0-1=Frame 0-1) (for BG Modes 4,5 only)
    display_frame_select: bool,
    /// Reserved/CGB Mode    (0=GBA, 1=CGB; can be set only by BIOS opcodes)
    reserved_cgb_mode: bool,
    /// Bg mode, in range 0..=5 (Bits 0..=2)
    bg_mode: BgMode,
}

/// | Mode | Rot/Scal | Layers | Size                                           | Tiles | Colours       | Features |
/// |------|----------|--------|------------------------------------------------|-------|---------------|----------|
/// | 0    | No       | 0123   | 256x256..512x515                               | 1024  | 16/16..256/1  | SFMABP   |
/// | 1    | Mixed    | 012-   | (BG0,BG1 as above Mode 0, BG2 as below Mode 2) |       |               |          |
/// | 2    | Yes      | --23   | 128x128..1024x1024                             | 256   | 256/1         | S-MABP   |
/// | 3    | Yes      | --2-   | 240x160                                        | 1     | 32768         | --MABP   |
/// | 4    | Yes      | --2-   | 240x160                                        | 2     | 256/1         | --MABP   |
/// | 5    | Yes      | --2-   | 160x128                                        | 2     | 32768         | --MABP   |
/// |      |          |        |                                                |       |               |          |
/// |      |          |        |                                                |       |               |          |
/// |      |          |        |                                                |       |               |          |
///
/// # Features
/// S)crolling, F)lip, M)osaic, A)lphaBlending, B)rightness, P)riority.
#[derive(Debug, BitfieldSpecifier)]
#[bits = 3]
pub enum BgMode {
    Mode0 = 0b000,
    Mode1 = 0b001,
    Mode2 = 0b010,
    Mode3 = 0b011,
    Mode4 = 0b100,
    Mode5 = 0b101,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct LcdStatus {
    /// The V-Count-Setting value is much the same as LYC of older gameboys.
    ///
    /// When its value is identical to the content of the VCOUNT register then the V-Counter flag is set (Bit 2), and (if enabled in Bit 5) an interrupt is requested.
    /// Although the drawing time is only 960 cycles (240*4), the H-Blank flag is "0" for a total of 1006 cycles.
    v_count_setting_lyc: u8,
    /// Unused in GBA, used for NDS
    #[skip]
    unused: B2,
    v_counter_irq_enable: bool,
    h_blank_irq_enable: bool,
    v_blank_irq_enable: bool,
    /// (Read only) (1=Match)  (set in selected line)
    v_counter_flag: bool,
    /// (Read only) (1=HBlank) (toggled in all lines, 0..227)
    h_blank_flag: bool,
    /// (Read only) (1=VBlank) (set in line 160..226; not 227)
    v_blank_flag: bool,
}

/// Indicates the currently drawn scanline
#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct VerticalCounter {
    /// Unused in GBA, for NDS bit 8 is used as most significant bit of scanline
    #[skip]
    unused: u8,
    /// (Read only) Current scanline (LY), has range (0..227)
    ///
    /// Values in range from 160..227 indicate 'hidden' scanlines within VBlank area.
    current_scanline: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgControl {
    /// 14..=15
    ///
    /// | Value | Text Mode    | Rotation/Scaling Mode |
    /// |-------|--------------|-----------------------|
    /// | 0     | 256x256 (2K) | 128x128               |
    /// | 1     | 512x256 (4K) | 256x256               |
    /// | 2     | 256x512 (4K) | 512x512               |
    /// | 3     | 512x512 (8K) | 1024x1024 (16K)       |
    screen_size: B2,
    /// Bit 13
    ///
    /// For Bg2/Bg3: Display Area Overflow (0=Transparent, 1=Wraparound)
    /// For Bg0/Bg1: Not used (except in NDS mode: Ext Palette Slot for Bg0/Bg1)
    display_area_overflow: bool,
    /// 8..=12  
    ///
    /// (0-31, in units of 2 KBytes) (=BG Map Data)
    screen_base_block: B5,
    /// Bit 7
    ///
    /// (0=16/16, 1=256/1)
    colors_palettes: bool,
    /// Bit 6
    ///
    /// (0=Disable, 1=Enable)
    mosaic: bool,
    /// 4..=5
    ///
    /// Not used (must be zero) (except in NDS mode: MSBs of char base)
    #[skip]
    unused: B2,
    /// 2..=3
    ///
    /// (0-3, in units of 16 KBytes) (=BG Tile Data)
    character_base_block: B2,
    /// 0..=1
    ///
    /// (0-3, 0=Highest)
    bg_priority: B2,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgScrolling {
    #[skip]
    unused: B7,
    /// Offset 0..=511
    offset: B9,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgRotationRef {
    #[skip]
    unused: B4,
    sign: bool,
    integer_portion: B19,
    fractional_portion: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgRotationParam {
    sign: bool,
    integer_portion: B7,
    fractional_portion: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct WindowDimensions {
    /// Leftmost coordinate of window
    /// OR
    /// Top-most coordinate of window
    left_top_most: u8,
    /// Rightmost coordinate of window, plus 1
    /// OR
    /// Bottom-most coordinate of window, plus 1
    right_bottom_most: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct WindowControl {
    #[skip]
    unused_1: B2,
    winobj_1_color_special: bool,
    winobj_1_obj_enable: bool,
    winobj_1_bg_enable: B4,

    #[skip]
    unused_0: B2,
    winout_0_color_special: bool,
    winout_0_obj_enable: bool,
    winout_0_bg_enable: B4,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct MosaicFunction {
    #[skip]
    unused: u16,
    obj_mosaic_v_size: B4,
    obj_mosaic_h_size: B4,
    bg_mosaic_v_size: B4,
    bg_mosaic_h_size: B4,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct ColorSpecialSelection {
    #[skip]
    unused: B2,
    bd_2: bool,
    obj_2: bool,
    bg3_2: bool,
    bg2_2: bool,
    bg1_2: bool,
    bg0_2: bool,
    color_special_effects: ColorSpecialEffect,
    bd_1: bool,
    obj_1: bool,
    bg3_1: bool,
    bg2_1: bool,
    bg1_1: bool,
    bg0_1: bool,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 2]
enum ColorSpecialEffect {
    None = 0b00,
    AlphaBlending = 0b01,
    BrightnessIncrease = 0b10,
    BrightnessDecrease = 0b11,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct AlphaBlendCoefficients {
    #[skip]
    unused: B3,
    evb: B5,
    #[skip]
    unused_1: B3,
    eva: B5,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BrightnessCoefficients {
    #[skip]
    unused: B27,
    evy: B5,
}
