use modular_bitfield_msb::prelude::{B19, B2, B27, B3, B4, B5, B7, B9};
use modular_bitfield_msb::{bitfield, BitfieldSpecifier};

use crate::emulator::MemoryAddress;

pub const LCD_CONTROL_START: MemoryAddress = 0x0400_0000;
pub const LCD_CONTROL_END: MemoryAddress = 0x0400_0001;
/// Note, we don't bother with emulating this register as nothing uses it.
pub const GREEN_SWAP_START: MemoryAddress = 0x0400_0002;
pub const GREEN_SWAP_END: MemoryAddress = 0x0400_0003;
pub const LCD_STATUS_START: MemoryAddress = 0x0400_0004;
pub const LCD_STATUS_END: MemoryAddress = 0x0400_0005;
pub const LCD_VERTICAL_COUNTER_START: MemoryAddress = 0x0400_0006;
pub const LCD_VERTICAL_COUNTER_END: MemoryAddress = 0x0400_0007;

crate::bitfield_update!(
    LcdControl,
    LcdStatus,
    VerticalCounter,
    BgControl,
    BgRotationRef,
    BgRotationParam,
    BgScrolling,
    WindowControl,
    WindowDimensions,
    MosaicFunction,
    ColorSpecialSelection,
    AlphaBlendCoefficients,
    BrightnessCoefficients
);

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct LcdControl {
    /// OBJ Window Display Flag (0=Off, 1=On) (Bit 15)
    pub obj_window_display: bool,
    /// Window 1 Display Flag   (0=Off, 1=On)
    pub window_1_display_flag: bool,
    /// Window 0 Display Flag   (0=Off, 1=On)
    pub window_0_display_flag: bool,
    /// Screen Display OBJ  (0=Off, 1=On)
    pub screen_display_obj: bool,
    /// Screen Display BG3  (0=Off, 1=On)
    pub screen_display_bg3: bool,
    /// Screen Display BG2  (0=Off, 1=On)
    pub screen_display_bg2: bool,
    /// Screen Display BG1  (0=Off, 1=On)
    pub screen_display_bg1: bool,
    /// Screen Display BG0  (0=Off, 1=On)
    pub screen_display_bg0: bool,
    /// Forced blank (1=Allow FAST access to VRAM,Palette,OAM)
    pub forced_blank: bool,
    /// OBJ Character VRAM Mapping (0=Two dimensional, 1=One dimensional)
    pub obj_character_vram_mapping: bool,
    /// H-Blank Interval Free  (1=Allow access to OAM during H-Blank)
    pub h_blank_interval_free: bool,
    /// Display Frame Select   (0-1=Frame 0-1) (for BG Modes 4,5 only)
    pub display_frame_select: bool,
    /// Reserved/CGB Mode    (0=GBA, 1=CGB; can be set only by BIOS opcodes)
    pub reserved_cgb_mode: bool,
    /// Bg mode, in range 0..=5 (Bits 0..=2)
    pub bg_mode: BgMode,
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
    pub v_count_setting_lyc: u8,
    /// Unused in GBA, used for NDS
    #[skip]
    unused: B2,
    pub v_counter_irq_enable: bool,
    pub h_blank_irq_enable: bool,
    pub v_blank_irq_enable: bool,
    /// (Read only) (1=Match)  (set in selected line)
    pub v_counter_flag: bool,
    /// (Read only) (1=HBlank) (toggled in all lines, 0..227)
    pub h_blank_flag: bool,
    /// (Read only) (1=VBlank) (set in line 160..226; not 227)
    pub v_blank_flag: bool,
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
    pub current_scanline: u8,
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
    pub screen_size: B2,
    /// Bit 13
    ///
    /// For Bg2/Bg3: Display Area Overflow (0=Transparent, 1=Wraparound)
    /// For Bg0/Bg1: Not used (except in NDS mode: Ext Palette Slot for Bg0/Bg1)
    pub display_area_overflow: bool,
    /// 8..=12  
    ///
    /// (0-31, in units of 2 KBytes) (=BG Map Data)
    pub screen_base_block: B5,
    /// Bit 7
    ///
    /// (0=16/16, 1=256/1)
    pub colors_palettes: bool,
    /// Bit 6
    ///
    /// (0=Disable, 1=Enable)
    pub mosaic: bool,
    /// 4..=5
    ///
    /// Not used (must be zero) (except in NDS mode: MSBs of char base)
    #[skip]
    unused: B2,
    /// 2..=3
    ///
    /// (0-3, in units of 16 KBytes) (=BG Tile Data)
    pub character_base_block: B2,
    /// 0..=1
    ///
    /// (0-3, 0=Highest)
    pub bg_priority: B2,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgScrolling {
    #[skip]
    unused: B7,
    /// Offset 0..=511
    pub offset: B9,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgRotationParam {
    #[skip]
    unused: B4,
    pub sign: bool,
    pub integer_portion: B19,
    pub fractional_portion: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BgRotationRef {
    pub sign: bool,
    pub integer_portion: B7,
    pub fractional_portion: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct WindowDimensions {
    /// Leftmost coordinate of window
    /// OR
    /// Top-most coordinate of window
    pub left_top_most: u8,
    /// Rightmost coordinate of window, plus 1
    /// OR
    /// Bottom-most coordinate of window, plus 1
    pub right_bottom_most: u8,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct WindowControl {
    #[skip]
    unused_1: B2,
    pub winobj_1_color_special: bool,
    pub winobj_1_obj_enable: bool,
    pub winobj_1_bg_enable: B4,

    #[skip]
    unused_0: B2,
    pub winout_0_color_special: bool,
    pub winout_0_obj_enable: bool,
    pub winout_0_bg_enable: B4,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct MosaicFunction {
    #[skip]
    unused: u16,
    pub obj_mosaic_v_size: B4,
    pub obj_mosaic_h_size: B4,
    pub bg_mosaic_v_size: B4,
    pub bg_mosaic_h_size: B4,
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct ColorSpecialSelection {
    #[skip]
    unused: B2,
    pub bd_2: bool,
    pub obj_2: bool,
    pub bg3_2: bool,
    pub bg2_2: bool,
    pub bg1_2: bool,
    pub bg0_2: bool,
    pub color_special_effects: ColorSpecialEffect,
    pub bd_1: bool,
    pub obj_1: bool,
    pub bg3_1: bool,
    pub bg2_1: bool,
    pub bg1_1: bool,
    pub bg0_1: bool,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 2]
pub enum ColorSpecialEffect {
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
    pub evb: B5,
    #[skip]
    unused_1: B3,
    pub eva: B5,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct BrightnessCoefficients {
    #[skip]
    unused: B27,
    pub evy: B5,
}
