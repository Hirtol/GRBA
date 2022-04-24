use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::frame::RgbaFrame;
use crate::emulator::ppu::oam::OamRam;
use crate::emulator::ppu::palette::PaletteRam;
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

#[cfg(feature = "debug-functionality")]
mod debug;
mod memory;
mod oam;
mod palette;
pub(crate) mod registers;
mod tile_rendering;

pub type PaletteIndex = u16;

#[derive(Default, Debug, Clone, Copy)]
pub struct BgScrollingCollection {
    pub x: BgScrolling,
    pub y: BgScrolling,
}

#[derive(Debug, Clone)]
pub struct PPU {
    // Ram
    frame_buffer: RgbaFrame,
    current_scanline: Box<[PaletteIndex; DISPLAY_WIDTH as usize]>,
    palette: PaletteRam,
    oam_ram: OamRam,
    vram: Box<[u8; VRAM_SIZE]>,

    // Registers
    disp_cnt: LcdControl,
    /// Not emulated
    green_swap: u16,
    disp_stat: LcdStatus,
    vertical_counter: VerticalCounter,
    /// The background control registers, for backgrounds 0..=3
    bg_control: [BgControl; 4],
    /// The background scrolling/offset registers, where `[0]` is X, and `[1]` is Y when indexing a particular background
    bg_scrolling: [BgScrollingCollection; 4],
    /// The background rotation references, where `[0]` is `BG2`, and `[1]` is `BG3`
    bg_rotation_x: [BgRotationParam; 2],
    bg_rotation_y: [BgRotationParam; 2],
    /// Internal background rotation/scaling for `BG2`
    ///
    /// Where the indexes correspond to the registers in the following way:
    /// * `[0]` is `PA`
    /// * `[1]` is `PB`
    /// * `[2]` is `PC`
    /// * `[3]` is `PD`
    bg_rotation_reference_bg2: [BgRotationRef; 4],
    /// Internal background rotation/scaling for `BG3`
    ///
    /// Where the indexes correspond to the registers in the following way:
    /// * `[0]` is `PA`
    /// * `[1]` is `PB`
    /// * `[2]` is `PC`
    /// * `[3]` is `PD`
    bg_rotation_reference_bg3: [BgRotationRef; 4],

    window_horizontal: [WindowDimensions; 2],
    window_vertical: [WindowDimensions; 2],
    window_control_inside: WindowControl,
    window_control_outside: WindowControl,

    mosaic_function: MosaicFunction,
    bld_cnt: ColorSpecialSelection,
    alpha: AlphaBlendCoefficients,
    brightness: BrightnessCoefficients,
}

impl PPU {
    pub fn new() -> Self {
        PPU {
            frame_buffer: RgbaFrame::default(),
            current_scanline: crate::box_array![0; DISPLAY_WIDTH as usize],
            palette: PaletteRam::default(),
            oam_ram: OamRam::default(),
            vram: crate::box_array![0; VRAM_SIZE],
            disp_cnt: LcdControl::new(),
            green_swap: 0,
            disp_stat: LcdStatus::new(),
            vertical_counter: VerticalCounter::new(),
            bg_control: [BgControl::new(); 4],
            bg_scrolling: [BgScrollingCollection::default(); 4],
            bg_rotation_x: [BgRotationParam::new(); 2],
            bg_rotation_y: [BgRotationParam::new(); 2],
            bg_rotation_reference_bg2: [BgRotationRef::new(); 4],
            bg_rotation_reference_bg3: [BgRotationRef::new(); 4],
            window_horizontal: [WindowDimensions::new(); 2],
            window_vertical: [WindowDimensions::new(); 2],
            window_control_inside: WindowControl::new(),
            window_control_outside: WindowControl::new(),
            mosaic_function: MosaicFunction::new(),
            bld_cnt: ColorSpecialSelection::new(),
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
        self.disp_stat.set_h_blank_flag(true);

        // Schedule HBlank interrupt if it's desired
        if self.disp_stat.h_blank_irq_enable() {
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
        self.disp_stat.set_h_blank_flag(false);

        self.vertical_counter
            .set_current_scanline(self.vertical_counter.current_scanline() + 1);

        // For handling VBlank ending (due to the fact that we increment the vertical_counter here putting this on the
        // scheduler is more difficult.
        if self.vertical_counter.current_scanline() == 227 {
            // Vblank is no longer set one hblank before the wrap-around
            self.disp_stat.set_v_blank_flag(false);
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
        self.disp_stat.set_v_blank_flag(true);

        if self.disp_stat.v_blank_irq_enable() {
            interrupts.request_interrupt(Interrupts::Vblank, scheduler);
        }
    }

    fn check_vertical_counter_interrupt(&mut self, scheduler: &mut Scheduler, interrupts: &mut InterruptManager) {
        if self.vertical_counter.current_scanline() == self.disp_stat.v_count_setting_lyc() {
            self.disp_stat.set_v_counter_flag(true);

            if self.disp_stat.v_counter_irq_enable() {
                interrupts.request_interrupt(Interrupts::VCounter, scheduler);
            }
        } else {
            self.disp_stat.set_v_counter_flag(false);
        }
    }

    fn render_scanline(&mut self) {
        // Only really relevant for Mode0..=2
        match self.disp_cnt.bg_mode() {
            BgMode::Mode0 => render_scanline_mode0(self),
            BgMode::Mode1 => {}
            BgMode::Mode2 => {}
            BgMode::Mode3 => {
                // Due to how we implement rendering we rely on palette indexes in the `current_scanline`.
                // For mode 3 we therefore render directly to the framebuffer, but we therefore need to do an early return.
                render_scanline_mode3(self);
                return;
            }
            BgMode::Mode4 => render_scanline_mode4(self),
            BgMode::Mode5 => {}
        }

        // May want to do this during HBlank if games use mid-scanline writes like in the GB
        self.push_current_scanline_to_framebuffer();
    }

    #[inline]
    fn push_current_scanline_to_framebuffer(&mut self) {
        let current_address: usize = self.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize;
        let framebuffer_slice = &mut self.frame_buffer[current_address..current_address + DISPLAY_WIDTH as usize];

        // Copy the values of the current scanline to the framebuffer.
        //TODO: Should backdrop color (palette index 0) be based on the highest-priority BG or the absolute palette 0?
        for (i, pixel) in self.current_scanline.iter().enumerate() {
            framebuffer_slice[i] = self.palette.get_palette(*pixel as usize).to_rgba(255);
        }

        self.current_scanline.fill(0);
    }

    pub fn frame_buffer(&mut self) -> &mut RgbaFrame {
        &mut self.frame_buffer
    }

    pub fn palette_cache(&self) -> &PaletteRam {
        &self.palette
    }
}

pub fn render_scanline_mode0(ppu: &mut PPU) {
    for priority in 0..4 {
        if ppu.disp_cnt.screen_display_bg0() {
            if ppu.bg_control[0].bg_priority() == priority {
                tile_rendering::render_scanline_regular_bg_pixel(ppu, 0);
            }
        }

        if ppu.disp_cnt.screen_display_bg1() {
            if ppu.bg_control[1].bg_priority() == priority {
                tile_rendering::render_scanline_regular_bg_pixel(ppu, 1);
            }
        }

        if ppu.disp_cnt.screen_display_bg2() {
            if ppu.bg_control[2].bg_priority() == priority {
                tile_rendering::render_scanline_regular_bg_pixel(ppu, 2);
            }
        }

        if ppu.disp_cnt.screen_display_bg3() {
            if ppu.bg_control[3].bg_priority() == priority {
                tile_rendering::render_scanline_regular_bg_pixel(ppu, 3);
            }
        }
    }
}

#[profiling::function]
pub fn render_scanline_mode3(ppu: &mut PPU) {
    let vram_index = ppu.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize;

    let cur_frame_addr: usize = ppu.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize;
    // Since this mode is a little special we directly render to the framebuffer, as we don't get paletted to index.
    let framebuffer_slice = &mut ppu.frame_buffer[cur_frame_addr..cur_frame_addr + DISPLAY_WIDTH as usize];
    for i in 0..DISPLAY_WIDTH as usize {
        // * 2 since we're rendering one pixel per two bytes
        let index = (vram_index + i) * 2;
        let pixel = u16::from_le_bytes(ppu.vram[index..=index + 1].try_into().unwrap());

        framebuffer_slice[i] = RGBA {
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
    let vram_index_base = ppu.disp_cnt.display_frame_select() as usize * FRAME_1_ADDR;

    let vram_index = vram_index_base + (ppu.vertical_counter.current_scanline() as usize * DISPLAY_WIDTH as usize);

    for i in 0..DISPLAY_WIDTH as usize {
        let palette_index = ppu.vram[vram_index + i];
        // Background palettes are always located in the first 256 bytes of the palette ram.
        ppu.current_scanline[i] = palette::convert_bg_to_absolute_palette(palette_index);
    }
}
