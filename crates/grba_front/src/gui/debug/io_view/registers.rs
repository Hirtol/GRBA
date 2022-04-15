use std::ops::RangeInclusive;

use egui::{Ui, Widget};
use enum_iterator::IntoEnumIterator;
use itertools::Itertools;
use once_cell::sync::Lazy;

use grba_core::emulator::debug::{
    BgMode, BG_CONTROL_START, BG_SCROLL_START, LCD_CONTROL_END, LCD_CONTROL_START, LCD_STATUS_END, LCD_STATUS_START,
};
use grba_core::emulator::MemoryAddress;

use crate::gui::debug::io_view::io_utils;

macro_rules! offset {
    ($start:expr, $offset:expr) => {
        $start + $offset..=$start + $offset + 1
    };
}

/// Ideally this would just be `const`, however, until `&mut` in `fn` is stable we can't have `draw` calls in the
/// [IoView] object const fn.
pub static IO_REGISTER_VIEWS: Lazy<[IoView; 17]> = Lazy::new(|| {
    [
        IoView::new_16("IEnable", 0x04000200..=0x04000201, draw_ie_if_view),
        IoView::new_16("IFlags", 0x04000202..=0x04000203, draw_ie_if_view),
        IoView::new_32("IME", 0x04000208..=0x0400020B, draw_ime_view),
        IoView::new_16("DispCnt", LCD_CONTROL_START..=LCD_CONTROL_END, draw_disp_cnt),
        IoView::new_16("DispStat", LCD_STATUS_START..=LCD_STATUS_END, draw_disp_stat_view),
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

    let items = BgMode::into_enum_iter().map(|m| format!("{:?}", m)).collect_vec();
    changed |= io_utils::io_list(ui, &mut reg_value, 0..=2, "Bg Mode", &*items);

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

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
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

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
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

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
}

fn draw_bg_scroll_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u16::from_le_bytes(reg_value.try_into().unwrap()) as u32;

    changed |= io_utils::io_slider(ui, &mut reg_value, 0x0..=0x8, "BG Scroll", 0..=511);

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
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

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
}

fn draw_ime_view(ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
    let mut changed = false;
    let mut reg_value = u32::from_le_bytes(reg_value.try_into().unwrap());

    changed |= io_utils::io_checkbox(ui, &mut reg_value, 0x0, "Interrupt Master Enable");

    if changed {
        Some(reg_value.to_le_bytes().into())
    } else {
        None
    }
}

fn format_u16(reg_value: &[u8]) -> String {
    format!("{:#06X}", u16::from_le_bytes(reg_value.try_into().unwrap()))
}

fn format_u32(reg_value: &[u8]) -> String {
    format!("{:#010X}", u32::from_le_bytes(reg_value.try_into().unwrap()))
}
