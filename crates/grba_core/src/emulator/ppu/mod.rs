use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::ppu::registers::{
    AlphaBlendCoefficients, BgControl, BgRotationParam, BgRotationRef, BgScrolling, BrightnessCoefficients,
    ColorSpecialSelection, LcdControl, LcdStatus, MosaicFunction, VerticalCounter, WindowControl, WindowDimensions,
};
use crate::scheduler::{EmuTime, EventTag, Scheduler};
pub use memory::*;

pub const DISPLAY_WIDTH: u32 = 240;
pub const DISPLAY_HEIGHT: u32 = 160;
pub const VRAM_SIZE: usize = 96 * 1024;
pub const PALETTE_RAM_SIZE: usize = 1024;
pub const OAM_RAM_SIZE: usize = 1024;

pub const CYCLES_PER_PIXEL: u32 = 4;
/// 960 Cycles per drawing scanline
pub const HDRAW_CYCLES: u32 = DISPLAY_WIDTH * CYCLES_PER_PIXEL;
pub const HBLANK_CYCLES: u32 = 272;
pub const SCANLINE_CYCLES: u32 = HDRAW_CYCLES + HBLANK_CYCLES;
pub const VBLANK_CYCLES: u32 = 83776;
pub const FRAME_CYCLES: u32 = 280896;

// 15 bit colour
// 96KB of VRAM
// 256 BG palette and 256 OBJ palette
// Transparency defined (RGBA)
// 8x8 tiles
// has direct bitmap modes
// 128 sprites can be on screen at the same time
// Sprites can go up to 64x64 (not useful)
// 6 video modes:
// * Mode 0..=2: Tiles modes
// * Mode 3..=5: Bitmap modes
// One frame is 280896 cycles

mod memory;
mod registers;

#[derive(Debug, Clone)]
pub struct PPU {
    // Ram
    palette_ram: Box<[u8; PALETTE_RAM_SIZE]>,
    oam_ram: Box<[u8; OAM_RAM_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,

    // Registers
    control: LcdControl,
    /// Not emulated
    green_swap: u16,
    status: LcdStatus,
    vertical_counter: VerticalCounter,
    /// The background control registers, for backgrounds 0..=3
    bg_control: [BgControl; 4],
    /// The background scrolling/offset registers, where `[0]` is X, and `[1]` is Y when indexing a particular background
    bg_offset: [[BgScrolling; 2]; 4],
    /// The background rotation references, where `[0]` is `BG2`, and `[1]` is `BG3`
    bg_rotation_x: [BgRotationParam; 2],
    bg_rotation_y: [BgRotationParam; 2],
    /// Internal background rotation/scaling for `BG2`
    bg_rotation_reference_bg2: [BgRotationRef; 4],
    /// Internal background rotation/scaling for `BG3`
    bg_rotation_reference_bg3: [BgRotationRef; 4],

    window_horizontal: [WindowDimensions; 2],
    window_vertical: [WindowDimensions; 2],
    window_control_inside: WindowControl,
    window_control_outside: WindowControl,

    mosaic_function: MosaicFunction,
    special: ColorSpecialSelection,
    alpha: AlphaBlendCoefficients,
    brightness: BrightnessCoefficients,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            palette_ram: crate::box_array![0; PALETTE_RAM_SIZE],
            oam_ram: crate::box_array![0; OAM_RAM_SIZE],
            vram: crate::box_array![0; VRAM_SIZE],
            control: LcdControl::new(),
            green_swap: 0,
            status: LcdStatus::new(),
            vertical_counter: VerticalCounter::new(),
            bg_control: [BgControl::new(); 4],
            bg_offset: [[BgScrolling::new(); 2]; 4],
            bg_rotation_x: [BgRotationParam::new(); 2],
            bg_rotation_y: [BgRotationParam::new(); 2],
            bg_rotation_reference_bg2: [BgRotationRef::new(); 4],
            bg_rotation_reference_bg3: [BgRotationRef::new(); 4],
            window_horizontal: [WindowDimensions::new(); 2],
            window_vertical: [WindowDimensions::new(); 2],
            window_control_inside: WindowControl::new(),
            window_control_outside: WindowControl::new(),
            mosaic_function: MosaicFunction::new(),
            special: ColorSpecialSelection::new(),
            alpha: AlphaBlendCoefficients::new(),
            brightness: BrightnessCoefficients::new(),
        }
    }

    pub fn hblank_start(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        self.status.set_h_blank_flag(true);

        // Schedule HBlank interrupt if it's desired
        if self.status.h_blank_irq_enable() {
            interrupts.request_interrupt(Interrupts::Hblank, scheduler);
        }

        // Render a scanline if we're not yet at the final line
        if self.vertical_counter.current_scanline() < (DISPLAY_HEIGHT - 1) as u8 {
            self.render_scanline();
        }

        scheduler.schedule_relative(EventTag::HBlankEnd, EmuTime::from(HBLANK_CYCLES));
    }

    pub fn hblank_end(&mut self, scheduler: &mut Scheduler) {
        self.status.set_h_blank_flag(false);

        if self.vertical_counter.current_scanline() == (DISPLAY_HEIGHT - 1) as u8 {
            // Next up is vblank, starts at beginning of next scanline thus only HBLANK_CYCLES
            scheduler.schedule_relative(EventTag::VBlank, EmuTime::from(0u32));
        } else {
            scheduler.schedule_relative(EventTag::HBlank, EmuTime::from(HDRAW_CYCLES));
        }
    }

    pub fn vblank(&mut self, scheduler: &mut Scheduler) {}

    fn render_scanline(&self) {
        todo!()
    }
}
