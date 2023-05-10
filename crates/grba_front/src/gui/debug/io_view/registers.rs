use std::ops::RangeInclusive;

use egui::Ui;
use itertools::Itertools;
use once_cell::sync::Lazy;

use grba_core::emulator::bus::IO_START;
use grba_core::emulator::debug::{
    BgMode, BG_CONTROL_START, BG_SCROLL_START, LCD_CONTROL_END, LCD_CONTROL_START, LCD_STATUS_END, LCD_STATUS_START,
    VCOUNT_END, VCOUNT_START,
};
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::io_view::io_utils;

macro_rules! offset {
    ($start:expr, $offset:expr, $len:expr) => {
        $start + $offset..=$start + $offset + ($len - 1)
    };
    ($start:expr, $offset:expr) => {
        $start + $offset..=$start + $offset + 1
    };
}

/// Ideally this would just be `const`, however, until `&mut` in `fn` is stable we can't have `draw` calls in the
/// [IoView] object const fn.
pub static IO_REGISTER_VIEWS: Lazy<[IoView; 41]> = Lazy::new(|| {
    [
        IoView::new_16("IEnable", offset!(IO_START, 0x200), draw_ie_if_view),
        IoView::new_16("IFlags", offset!(IO_START, 0x202), draw_ie_if_view),
        IoView::new_32("IME", offset!(IO_START, 0x208, 4), draw_ime_view),
        IoView::new_16("IKeyCnt", offset!(IO_START, 0x132), draw_keypad_int_view),
        IoView::new_16("DispCnt", LCD_CONTROL_START..=LCD_CONTROL_END, draw_disp_cnt),
        IoView::new_16("DispStat", LCD_STATUS_START..=LCD_STATUS_END, draw_disp_stat_view),
        IoView::new_16("Vcount", VCOUNT_START..=VCOUNT_END, draw_v_count_view),
        IoView::new_16("Bg0Control", offset!(BG_CONTROL_START, 0), draw_bg_control_view),
        IoView::new_16("Bg1Control", offset!(BG_CONTROL_START, 2), draw_bg_control_view),
        IoView::new_16("Bg2Control", offset!(BG_CONTROL_START, 4), draw_bg_control_view),
        IoView::new_16("Bg3Control", offset!(BG_CONTROL_START, 6), draw_bg_control_view),
        IoView::new_16("Bg0HOFS", offset!(BG_SCROLL_START, 0), draw_bg_scroll_view),
        IoView::new_16("Bg0VOFS", offset!(BG_SCROLL_START, 2), draw_bg_scroll_view),
        IoView::new_16("Bg1HOFS", offset!(BG_SCROLL_START, 4), draw_bg_scroll_view),
        IoView::new_16("Bg1VOFS", offset!(BG_SCROLL_START, 6), draw_bg_scroll_view),
        IoView::new_16("Bg2HOFS", offset!(BG_SCROLL_START, 8), draw_bg_scroll_view),
        IoView::new_16("Bg2VOFS", offset!(BG_SCROLL_START, 10), draw_bg_scroll_view),
        IoView::new_16("Bg3HOFS", offset!(BG_SCROLL_START, 12), draw_bg_scroll_view),
        IoView::new_16("Bg3VOFS", offset!(BG_SCROLL_START, 14), draw_bg_scroll_view),
        IoView::new_16("Bg2PA", offset!(IO_START, 0x20), unimplemented_view),
        IoView::new_16("Bg2PB", offset!(IO_START, 0x22), unimplemented_view),
        IoView::new_16("Bg2PC", offset!(IO_START, 0x24), unimplemented_view),
        IoView::new_16("Bg2PD", offset!(IO_START, 0x26), unimplemented_view),
        IoView::new_32("Bg2X", offset!(IO_START, 0x28, 4), unimplemented_view),
        IoView::new_32("Bg2Y", offset!(IO_START, 0x2C, 4), unimplemented_view),
        IoView::new_16("Bg3PA", offset!(IO_START, 0x30), unimplemented_view),
        IoView::new_16("Bg3PB", offset!(IO_START, 0x32), unimplemented_view),
        IoView::new_16("Bg3PC", offset!(IO_START, 0x34), unimplemented_view),
        IoView::new_16("Bg3PD", offset!(IO_START, 0x36), unimplemented_view),
        IoView::new_32("Bg3X", offset!(IO_START, 0x38, 4), unimplemented_view),
        IoView::new_32("Bg3Y", offset!(IO_START, 0x3C, 4), unimplemented_view),
        IoView::new_16("Win0H", offset!(IO_START, 0x40), draw_window_horizontal_dim_view),
        IoView::new_16("Win0V", offset!(IO_START, 0x44), draw_window_vertical_dim_view),
        IoView::new_16("Win1H", offset!(IO_START, 0x42), draw_window_horizontal_dim_view),
        IoView::new_16("Win1V", offset!(IO_START, 0x46), draw_window_vertical_dim_view),
        IoView::new_16("WinIn", offset!(IO_START, 0x48), draw_winin_view),
        IoView::new_16("WinOut", offset!(IO_START, 0x4A), draw_winout_view),
        IoView::new_32("Mosaic", offset!(IO_START, 0x4C, 4), draw_mosaic_view),
        IoView::new_16("BldCnt", offset!(IO_START, 0x50), draw_bldcnt_view),
        IoView::new_16("BldAlpha", offset!(IO_START, 0x52), draw_bldalpha_view),
        IoView::new_16("BldY", offset!(IO_START, 0x54), draw_bldy_view),
    ]
});

/// A view of a single register
pub struct IoView {
    /// The name of the register.
    ///
    /// Used to display the register name in the UI.
    pub name: &'static str,
    /// The address range of the register.
    pub address: RangeInclusive<MemoryAddress>,
    /// Display the current register value as a hex coded `u8`/`u16`/`u32`.
    pub format: fn(reg_value: &[u8]) -> String,
    /// Draw the register content within the provided `ui`.
    ///
    /// The `reg_value` is the value of the memory located at [Self::get_address].
    ///
    /// # Returns
    ///
    /// [Some] if the register was changed by the user, [None] otherwise.
    pub draw: fn(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>>,
}

impl IoView {
    pub fn new(
        name: &'static str,
        address: RangeInclusive<MemoryAddress>,
        format: fn(reg_value: &[u8]) -> String,
        draw: fn(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>>,
    ) -> Self {
        IoView {
            name,
            address,
            format,
            draw,
        }
    }

    pub fn new_16(
        name: &'static str,
        address: RangeInclusive<MemoryAddress>,
        draw: fn(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>>,
    ) -> Self {
        IoView::new(name, address, format_u16, draw)
    }

    pub fn new_32(
        name: &'static str,
        address: RangeInclusive<MemoryAddress>,
        draw: fn(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>>,
    ) -> Self {
        IoView::new(name, address, format_u32, draw)
    }
}

fn draw_disp_cnt(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;

    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    let items = enum_iterator::all::<BgMode>().map(|m| format!("{:?}", m)).collect_vec();
    changed |= io_utils::io_list(ui, &mut reg_value, 0..=2, "Bg Mode", &items);

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "Display Frame Select (BG-Modes 4,5 only)");
    changed |= io_utils::io_checkbox(
        ui,
        &mut reg_value,
        0x5,
        "H-Blank Interval Free (Allow access to OAM during H-Blank)",
    );
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0x6..=0x6,
        "OBJ Character VRAM Mapping",
        &["2D", "1D"],
    );
    changed |= io_utils::io_checkbox(
        ui,
        &mut reg_value,
        0x7,
        "Forced blank (1=Allow FAST access to VRAM,Palette,OAM)",
    );
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "Screen Display BG0");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "Screen Display BG1");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xA, "Screen Display BG2");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xB, "Screen Display BG3");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xC, "Screen Display OBJ");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xD, "Window 0 Display Flag");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xE, "Window 1 Display Flag");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xF, "OBJ Window Display Flag");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_disp_stat_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "V Blank Flag");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "H Blank Flag");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "V Counter Flag");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "V Blank IRQ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "H Blank IRQ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "V Counter IRQ Enable");
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x8..=0xF, "V Count Setting LYC", 0..=255);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_v_count_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x7, "V Count", 0..=227);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_bg_control_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_radio(ui, &mut reg_value, 0x0..=0x1, "BG Priority");
    changed |= io_utils::io_radio(ui, &mut reg_value, 0x2..=0x3, "BG Tile Data (Val * 0x4000)");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x6, "Mosaic");
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0x7..=0x7,
        "Colors/Palettes",
        &["16/16 colours/palettes", "256/1 colours/palettes"],
    );
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x8..=0xC, "BG Map Data (Val * 0x800)", 0..=31);
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0xD..=0xD,
        "Display Area Overflow (BG2/BG3)",
        &["Transparent", "Wraparound"],
    );
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0xE..=0xF,
        "Screen Size",
        &[
            "256x256 (32x32 tiles)",
            "512x256 (64x32 tiles)",
            "256x512 (32x64 tiles)",
            "512x512 (64x64 tiles)",
        ],
    );

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_bg_scroll_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x8, "BG Scroll", 0..=511);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_ie_if_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "V Blank Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "H Blank Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "V Counter Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "Timer 0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "Timer 1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "Timer 2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x6, "Timer 3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x7, "Serial Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "DMA 0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "DMA 1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xA, "DMA 2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xB, "DMA 3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xC, "Keypad Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xD, "Game Pak IRQ Enable");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_ime_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u32::from_le_bytes(reg_value.try_into().unwrap());

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "Interrupt Master Enable");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_keypad_int_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "Button A");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "Button B");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "Button Select");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "Button Start");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "Button Right");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "Button Left");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x6, "Button Up");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x7, "Button Down");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "Shoulder Right");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "Shoulder Left");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xE, "Button IRQ Enable");
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0xF..=0xF,
        "Button IRQ Condition",
        &["Logical OR", "Logical AND"],
    );

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_window_horizontal_dim_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x7, "Right-most Coordinate + 1", 0..=255);
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x8..=0xF, "Left-most Coordinate", 0..=255);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_window_vertical_dim_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x7, "Bottom-most Coordinate + 1", 0..=255);
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x8..=0xF, "Top-most Coordinate", 0..=255);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_winin_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "Window 0 BG0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "Window 0 BG1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "Window 0 BG2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "Window 0 BG3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "Window 0 OBJ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "Window 0 Color Special Enable");

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "Window 1 BG0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "Window 1 BG1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xA, "Window 1 BG2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xB, "Window 1 BG3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xC, "Window 1 OBJ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xD, "Window 1 Color Special Enable");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_winout_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "Outside BG0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "Outside BG1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "Outside BG2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "Outside BG3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "Outside OBJ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "Outside Color Special Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "OBJ Window BG0 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "OBJ Window BG1 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xA, "OBJ Window BG2 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xB, "OBJ Window BG3 Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xC, "OBJ Window OBJ Enable");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xD, "OBJ Window Color Special Enable");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_mosaic_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u32::from_le_bytes(reg_value.try_into().unwrap());

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x3, "BG-H Size", 0..=15);
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x4..=0x7, "BG-V Size", 0..=15);
    changed |= io_utils::io_slider(ui, &mut reg_value, 0x8..=0xB, "OBJ-H Size", 0..=15);
    changed |= io_utils::io_slider(ui, &mut reg_value, 0xC..=0xF, "OBJ-V Size", 0..=15);

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_bldcnt_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "BG0 1st Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x1, "BG1 1st Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x2, "BG2 1st Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x3, "BG3 1st Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x4, "OBJ 1st Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x5, "Backdrop 1st Target Pixel");
    changed |= io_utils::io_list(
        ui,
        &mut reg_value,
        0x6..=0x7,
        "Color Special Effect",
        &[
            "None (Special Effects Disabled)",
            "Alpha Blending (1st+2nd Target Mix)",
            "Brightness Increase (1st Target Whiter)",
            "Brightness Decrease (1st Target Darker)",
        ],
    );
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x8, "BG0 2nd Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x9, "BG1 2nd Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xA, "BG2 2nd Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xB, "BG3 2nd Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xC, "OBJ 2nd Target Pixel");
    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0xD, "Backdrop 2nd Target Pixel");

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_bldalpha_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(
        ui,
        &mut reg_value,
        0x0..=0x4,
        "EVA (1st Target), 0..=16 = 0/16..=16/16, 17..=31 = 16/16",
        0..=31,
    );
    changed |= io_utils::io_slider(
        ui,
        &mut reg_value,
        0x8..=0x12,
        "EVB (2nd Target), 0..=16 = 0/16..=16/16, 17..=31 = 16/16",
        0..=31,
    );

    changed.then(|| reg_value.to_le_bytes().into())
}

fn draw_bldy_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(
        ui,
        &mut reg_value,
        0x0..=0x4,
        "EVY (Brightness), 0..=16 = 0/16..=16/16, 17..=31 = 16/16",
        0..=31,
    );

    changed.then(|| reg_value.to_le_bytes().into())
}

pub fn unimplemented_view(ui: &mut Ui, _reg_value: &[u8]) -> Option<Vec<u8>> {
    ui.label("Unimplemented");
    None
}

fn format_u16(reg_value: &[u8]) -> String {
    format!("{:#06X}", u16::from_le_bytes(reg_value.try_into().unwrap()))
}

fn format_u32(reg_value: &[u8]) -> String {
    format!("{:#010X}", u32::from_le_bytes(reg_value.try_into().unwrap()))
}
