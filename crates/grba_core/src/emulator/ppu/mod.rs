use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::frame::RgbaFrame;
use crate::emulator::ppu::palette::PaletteCache;
use crate::emulator::ppu::registers::{
    AlphaBlendCoefficients, BgControl, BgMode, BgRotationParam, BgRotationRef, BgScrolling, BrightnessCoefficients,
    ColorSpecialSelection, LcdControl, LcdStatus, MosaicFunction, VerticalCounter, WindowControl, WindowDimensions,
};
use crate::scheduler::{EmuTime, EventTag, Scheduler};
use crate::utils::BitOps;
pub use memory::*;
pub use palette::{Palette, RGBA};

pub const DISPLAY_WIDTH: u32 = 240;
pub const DISPLAY_HEIGHT: u32 = 160;
pub const FRAMEBUFFER_SIZE: u32 = DISPLAY_WIDTH * DISPLAY_HEIGHT;
pub const VRAM_SIZE: usize = 96 * 1024;
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
mod palette;
pub(crate) mod registers;

#[derive(Debug, Clone)]
pub struct PPU {
    // Ram
    frame_buffer: RgbaFrame,
    current_scanline: Box<[RGBA; DISPLAY_WIDTH as usize]>,
    palette: PaletteCache,
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
            frame_buffer: RgbaFrame::default(),
            current_scanline: crate::box_array![RGBA::default(); DISPLAY_WIDTH as usize],
            palette: PaletteCache::default(),
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

    /// Executed when the PPU is created to kick-start the PPU event chain.
    pub fn initial_startup(&self, scheduler: &mut Scheduler) {
        scheduler.schedule_event(EventTag::HBlank, EmuTime::from(HDRAW_CYCLES));
    }

    pub fn hblank_start(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        crate::cpu_log!("ppu-logging"; "HBlank fired!");
        self.status.set_h_blank_flag(true);

        // Schedule HBlank interrupt if it's desired
        if self.status.h_blank_irq_enable() {
            interrupts.request_interrupt(Interrupts::Hblank, scheduler);
        }

        // Render a scanline if we're not yet at the final line
        if self.vertical_counter.current_scanline() < DISPLAY_HEIGHT as u8 {
            self.render_scanline();
        }

        scheduler.schedule_relative(EventTag::HBlankEnd, EmuTime::from(HBLANK_CYCLES));
    }

    pub fn hblank_end(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        crate::cpu_log!("ppu-logging"; "HBlankEnd fired!");
        self.status.set_h_blank_flag(false);

        self.vertical_counter
            .set_current_scanline(self.vertical_counter.current_scanline() + 1);

        // For handling VBlank ending (due to the fact that we increment the vertical_counter here putting this on the
        // scheduler is more difficult.
        if self.vertical_counter.current_scanline() == 227 {
            // Vblank is no longer set one hblank before the wrap-around
            self.status.set_v_blank_flag(false);
        } else if self.vertical_counter.current_scanline() == 228 {
            // Reached the end of vblank, time to reset the scanline counter
            self.vertical_counter.set_current_scanline(0);
        }

        self.check_vertical_counter_interrupt(scheduler, interrupts);

        if self.vertical_counter.current_scanline() == DISPLAY_HEIGHT as u8 {
            // Next up is vblank
            scheduler.schedule_relative(EventTag::VBlank, EmuTime::from(0u32));
        }

        // HBlank continues on even during VBlank
        scheduler.schedule_relative(EventTag::HBlank, EmuTime::from(HDRAW_CYCLES));
    }

    pub fn vblank(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        crate::cpu_log!("ppu-logging"; "Vblank fired at time: {:?}", scheduler.current_time);
        self.status.set_v_blank_flag(true);

        if self.status.v_blank_irq_enable() {
            interrupts.request_interrupt(Interrupts::Vblank, scheduler);
        }
    }

    fn check_vertical_counter_interrupt(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        if self.vertical_counter.current_scanline() == self.status.v_count_setting_lyc() {
            self.status.set_v_counter_flag(true);

            if self.status.v_counter_irq_enable() {
                interrupts.request_interrupt(Interrupts::VCounter, scheduler);
            }
        } else {
            self.status.set_v_counter_flag(false);
        }
    }

    fn render_scanline(&mut self) {
        crate::cpu_log!("ppu-logging"; "Rendering scanline {} - Mode: {:?}", self.vertical_counter.current_scanline(), self.control.bg_mode());
        // TODO: Backdrop color (when no background has rendered a pixel there (all transparent) should be filled with palette 0)
        // Only really relevant for Mode0..=2
        match self.control.bg_mode() {
            BgMode::Mode0 => {}
            BgMode::Mode1 => {}
            BgMode::Mode2 => {}
            BgMode::Mode3 => render_scanline_mode3(self),
            BgMode::Mode4 => render_scanline_mode4(self),
            BgMode::Mode5 => {}
        }

        // May want to do this during HBlank if games use mid-scanline writes like in the GB
        self.push_current_scanline_to_framebuffer();
    }

    #[inline]
    fn push_current_scanline_to_framebuffer(&mut self) {
        let current_address: usize = self.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize;
        // Copy the value of the current scanline to the framebuffer.
        self.frame_buffer[current_address..current_address + DISPLAY_WIDTH as usize]
            .copy_from_slice(&*self.current_scanline);
    }

    pub fn frame_buffer(&mut self) -> &mut RgbaFrame {
        &mut self.frame_buffer
    }

    pub fn palette_cache(&self) -> &PaletteCache {
        &self.palette
    }
}

#[profiling::function]
pub fn render_scanline_mode3(ppu: &mut PPU) {
    let vram_index = ppu.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize;

    for i in 0..DISPLAY_WIDTH as usize {
        // * 2 since we're rendering one pixel per two bytes
        let index = (vram_index + i) * 2;
        let pixel = u16::from_le_bytes(ppu.vram[index..=index + 1].try_into().unwrap());

        ppu.current_scanline[i] = RGBA {
            red: palette::convert_5_to_8_bit_color(pixel.get_bits(0, 4) as u8),
            green: palette::convert_5_to_8_bit_color(pixel.get_bits(5, 9) as u8),
            blue: palette::convert_5_to_8_bit_color(pixel.get_bits(10, 14) as u8),
            alpha: 255,
        };
    }
}

/// Render a full scanline of mode 4.
#[profiling::function]
pub fn render_scanline_mode4(ppu: &mut PPU) {
    const FRAME_0_ADDR: usize = 0x0;
    const FRAME_1_ADDR: usize = 0xA000;

    // If Frame 1 is selected (`display_frame_select` is true) then the frame buffer is located at 0xA000, otherwise
    // it will point to 0x0 for FRAME_0 due to the multiplication.
    let vram_index_base = ppu.control.display_frame_select() as usize * FRAME_1_ADDR;

    let vram_index = vram_index_base + (ppu.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize);

    for i in 0..DISPLAY_WIDTH as usize {
        let palette_index = ppu.vram[vram_index + i];
        let palette = ppu.palette.get_palette(palette_index as usize);

        ppu.current_scanline[i] = palette.to_rgba(255);
    }
}
