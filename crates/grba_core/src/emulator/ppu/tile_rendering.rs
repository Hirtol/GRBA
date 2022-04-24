use modular_bitfield::bitfield;
use modular_bitfield::prelude::{B10, B4};

use crate::emulator::ppu::{palette, PPU};
use crate::utils::BitOps;
use crate::DISPLAY_WIDTH;

/// 32x32 entries for 256x256 pixels, multiple of these blocks can be strung together.
pub const BG_MAP_TEXT_SIZE: usize = 0x800;
pub const CHAR_BLOCK_SIZE: usize = 1024 * 16;

const TILE_WIDTH_PIXELS: u16 = 8;
const TILE_HEIGHT_PIXELS: u16 = 8;
const TILE_WIDTH_8BPP: u16 = 8;
const TILE_WIDTH_4BPP: u16 = 4;
const TILE_SIZE_8BPP: u32 = 64;
const TILE_SIZE_4BPP: u32 = 32;
/// The amount of tiles displayed per row, assuming no scrolling.
///
/// `= 30`
const DISPLAYED_TILES_PER_ROW: u16 = DISPLAY_WIDTH as u16 / TILE_WIDTH_PIXELS;

#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone)]
pub struct BgMapTextData {
    /// The tile index to use for the current entry
    pub tile_number: B10,
    pub horizontal_flip: bool,
    pub vertical_flip: bool,
    /// Specifies the high-nibble for palette indexing in `16/16` mode.
    ///
    /// Unused when in `256/1` mode.
    pub palette_number: B4,
}

/// Regular screen size recorded in BgXCnt
enum RegularScreenSize {
    _256X256 = 0x0,
    _512X256 = 0x1,
    _256X512 = 0x2,
    _512X512 = 0x3,
}

impl RegularScreenSize {
    #[inline]
    const fn from_u8(val: u8) -> RegularScreenSize {
        match val {
            0x0 => RegularScreenSize::_256X256,
            0x1 => RegularScreenSize::_512X256,
            0x2 => RegularScreenSize::_256X512,
            0x3 => RegularScreenSize::_512X512,
            _ => unreachable!(),
        }
    }

    #[inline]
    const fn tiles_wide(&self) -> u16 {
        match self {
            RegularScreenSize::_256X256 | RegularScreenSize::_256X512 => 32,
            RegularScreenSize::_512X256 | RegularScreenSize::_512X512 => 64,
        }
    }

    #[inline]
    const fn tiles_high(&self) -> u16 {
        match self {
            RegularScreenSize::_256X256 | RegularScreenSize::_512X256 => 32,
            RegularScreenSize::_256X512 | RegularScreenSize::_512X512 => 64,
        }
    }

    const fn total_map_size(&self) -> u16 {
        self.tiles_wide() * self.tiles_high() * 2
    }
}

#[inline]
pub fn render_scanline_regular_bg_pixel(ppu: &mut PPU, bg: usize) {
    let cnt = &ppu.bg_control[bg];
    let scrolling = &ppu.bg_scrolling[bg];
    let (x_scroll, y_scroll) = (scrolling.x.offset(), scrolling.y.offset());
    let screen_size = RegularScreenSize::from_u8(cnt.screen_size());
    let (x_max_px, y_max_px) = (
        screen_size.tiles_wide() * TILE_WIDTH_PIXELS,
        screen_size.tiles_high() * TILE_HEIGHT_PIXELS,
    );

    let tile_base = cnt.tile_data_base() as usize * CHAR_BLOCK_SIZE;
    let map_base = cnt.tile_map_base() as usize * BG_MAP_TEXT_SIZE;
    let is_8bpp = cnt.colors_palettes();

    let scanline_to_draw = (ppu.vertical_counter.current_scanline() as u16).wrapping_add(y_scroll) % y_max_px;

    let tile_line_y = scanline_to_draw % TILE_HEIGHT_PIXELS;
    // + ((x_scroll / TILE_WIDTH_PIXELS) as usize * 2) would be nice, but difficult as we would then need to check how
    // many pixels to skip in that tile.
    let map_base = map_base + ((scanline_to_draw / TILE_HEIGHT_PIXELS) as usize * 64);

    for i in 0..DISPLAY_WIDTH as usize {
        // If the current pixel has already been written to by a higher-priority background/sprite, skip it.
        if ppu.current_scanline[i] != 0 {
            continue;
        }

        let absolute_pixel_x_coord = (i + x_scroll as usize) % x_max_px as usize;
        let map_coord = {
            let map_coord = map_base + ((absolute_pixel_x_coord / TILE_WIDTH_PIXELS as usize) * 2);

            match screen_size {
                RegularScreenSize::_512X256 | RegularScreenSize::_512X512 if absolute_pixel_x_coord > 255 => {
                    // Due to the fact that screen base blocks are arrayed in sets of 32x32 tiles we need to add
                    // an offset to go to the next block once we cross into its territory.
                    // We subtract one map_line (32 x 2 bytes)s worth to get the correct map line.
                    map_coord + (BG_MAP_TEXT_SIZE - 64)
                }
                _ => map_coord,
            }
        };

        let map_item: BgMapTextData = u16::from_le_bytes(ppu.vram[map_coord..map_coord + 2].try_into().unwrap()).into();

        // For tile flipping
        let tile_y_coord = tile_line_y ^ (0b111 * map_item.vertical_flip() as u16);
        let tile_x_coord =
            (absolute_pixel_x_coord % TILE_WIDTH_PIXELS as usize) ^ (0b111 * map_item.horizontal_flip() as usize);
        let tile_num = map_item.tile_number() as u32;

        let palette_index = if is_8bpp {
            let tile_line_addr =
                tile_base + (tile_num * TILE_SIZE_8BPP + (tile_y_coord * TILE_WIDTH_8BPP) as u32) as usize;
            let tile_pixel_addr = tile_line_addr + tile_x_coord;
            let palette_index = ppu.vram[tile_pixel_addr];

            palette_index
        } else {
            let palette_base = map_item.palette_number() * 16;
            let tile_line_addr =
                tile_base + (tile_num * TILE_SIZE_4BPP + (tile_y_coord * TILE_WIDTH_4BPP) as u32) as usize;
            let tile_pixel_addr = tile_line_addr + (tile_x_coord / 2);
            let two_palette_indexes = ppu.vram[tile_pixel_addr];
            let palette_index = (two_palette_indexes >> ((tile_x_coord % 2) * 4)) & 0x0F;

            if palette_index != 0 {
                palette_index + palette_base
            } else {
                palette_index
            }
        };

        ppu.current_scanline[i] = palette::convert_bg_to_absolute_palette(palette_index);
    }
}

#[inline]
pub fn render_scanline_regular_bg(ppu: &mut PPU, bg: usize) {
    let cnt = &ppu.bg_control[bg];
    let scrolling = &ppu.bg_scrolling[bg];
    let (x_scroll, y_scroll) = (scrolling.x.offset(), scrolling.y.offset());
    let screen_size = RegularScreenSize::from_u8(cnt.screen_size());

    let tile_data_base = cnt.tile_data_base() as usize * CHAR_BLOCK_SIZE;
    let map_base = cnt.tile_map_base() as usize * BG_MAP_TEXT_SIZE;
    let is_8bpp = cnt.colors_palettes();

    let scanline_to_be_rendered = (ppu.vertical_counter.current_scanline() as u16).wrapping_add(y_scroll);
    // * 2 as each tile map entry is 2 bytes. TODO: Verify map layout for larger (32x64, 64x32, 64x64) screens.
    let tile_map_entries = screen_size.tiles_wide() * 2;
    let tile_lower_bound: u16 =
        ((scanline_to_be_rendered / TILE_WIDTH_PIXELS) * tile_map_entries) + (x_scroll / TILE_WIDTH_8BPP);
    let mut tile_higher_bound = tile_lower_bound + DISPLAYED_TILES_PER_ROW * 2;

    // Which particular y coordinate to use from the tiles this scanline.
    let tile_line_y = scanline_to_be_rendered % TILE_WIDTH_8BPP;
    let mut pixels_drawn: i16 = 0;
    let mut pixels_to_skip: u16 = x_scroll % TILE_WIDTH_8BPP;

    // If the tile is not nicely aligned on % 8 boundaries we'll need an additional tile for the
    // last 8-pixels_to_skip pixels of the scanline.
    if pixels_to_skip > 0 {
        tile_higher_bound += 2;
    }

    let total_tile_map_size = screen_size.total_map_size();

    for mut i in (tile_lower_bound..=tile_higher_bound).step_by(2) {
        // When we wraparound in the x direction we want to stay on the same internal y-tile
        // Since we have a 1d representation of the tile map we have to subtract `tile_map_entries` to 'negate'
        // the effect of the x wraparound (since this wraparound
        // would have us go to the next y-tile line in the tile map)
        if (x_scroll + pixels_drawn as u16) >= (screen_size.tiles_wide() * TILE_WIDTH_4BPP) {
            // i -= tile_map_entries;
        }

        let addr = map_base + (i % total_tile_map_size) as usize;
        let tile_map_item: BgMapTextData = u16::from_le_bytes(ppu.vram[addr..=(addr + 1)].try_into().unwrap()).into();

        draw_bg_line(
            ppu,
            &mut pixels_drawn,
            &mut pixels_to_skip,
            tile_data_base,
            tile_line_y,
            tile_map_item,
            is_8bpp,
        );
    }
}

#[inline(always)]
fn draw_bg_line(
    ppu: &mut PPU,
    pixels_drawn: &mut i16,
    pixels_to_skip: &mut u16,
    tile_base_addr: usize,
    tile_line_y: u16,
    item: BgMapTextData,
    is_8bpp: bool,
) {
    let tile_num = item.tile_number() as u32;
    // For tile vertical flipping
    let tile_line_y = tile_line_y ^ (0b111 * item.vertical_flip() as u16);
    //TODO: Do full pixel line in one go if pixels_to_skip == 0 && pixels_drawn < DISPLAY_WIDTH - 8
    if is_8bpp {
        let addr = tile_base_addr + (tile_num * TILE_SIZE_8BPP + (tile_line_y * TILE_WIDTH_8BPP) as u32) as usize;
        let data = &ppu.vram[addr..(addr + TILE_WIDTH_8BPP as usize)];

        if item.horizontal_flip() {
            for palette_index in data.into_iter().copied().rev() {
                // TODO: Use iterator skip() and benchmark.
                if *pixels_to_skip > 0 {
                    *pixels_to_skip -= 1;
                    continue;
                }
                // Once we've reached the max width we can just stop.
                if *pixels_drawn >= DISPLAY_WIDTH as i16 {
                    break;
                }

                ppu.current_scanline[*pixels_drawn as usize] = palette::convert_bg_to_absolute_palette(palette_index);
                *pixels_drawn += 1;
            }
        } else {
            for palette_index in data.into_iter().copied() {
                // TODO: Use iterator skip() and benchmark.
                if *pixels_to_skip > 0 {
                    *pixels_to_skip -= 1;
                    continue;
                }
                // Once we've reached the max width we can just stop.
                if *pixels_drawn >= DISPLAY_WIDTH as i16 {
                    break;
                }

                ppu.current_scanline[*pixels_drawn as usize] = palette::convert_bg_to_absolute_palette(palette_index);
                *pixels_drawn += 1;
            }
        }
    } else {
        let addr = tile_base_addr + (tile_num * TILE_SIZE_4BPP + (tile_line_y * TILE_WIDTH_4BPP) as u32) as usize;
        let data = &ppu.vram[addr..(addr + TILE_WIDTH_4BPP as usize)];
        // Since we're in 4BPP mode we have 16x16 palettes, this determines the first.
        let palette_base = item.palette_number() * 16;

        if item.horizontal_flip() {
            for two_pixels in data.into_iter().copied().rev() {
                // TODO: Use iterator skip() and benchmark.
                if *pixels_to_skip > 0 {
                    if *pixels_to_skip == 1 {
                        // We should skip the first pixel in the pair
                        *pixels_to_skip = 0;
                        let last_pixel = two_pixels.get_bits(4, 7);
                        // We want to just set 0
                        let palette_index = if last_pixel != 0 { palette_base + last_pixel } else { 0 };

                        ppu.current_scanline[*pixels_drawn as usize] =
                            palette::convert_bg_to_absolute_palette(palette_index);
                        *pixels_drawn += 1;
                    } else {
                        *pixels_to_skip -= 2;
                    }
                } else {
                    // Since we're drawing 2 pixels at a time we need to check if we're near the max
                    if *pixels_drawn >= (DISPLAY_WIDTH - 1) as i16 {
                        if *pixels_drawn == DISPLAY_WIDTH as i16 {
                            break;
                        } else {
                            // Draw one last pixel
                            //TODO: Verify if 0..=3 or 4..=7
                            let palette_index = palette_base + two_pixels.get_bits(0, 3);
                            ppu.current_scanline[*pixels_drawn as usize] =
                                palette::convert_bg_to_absolute_palette(palette_index);
                            *pixels_drawn += 1;
                            break;
                        }
                    }

                    let (pal_1, pal_2) = (
                        palette_base + two_pixels.get_bits(0, 3),
                        palette_base + two_pixels.get_bits(4, 7),
                    );

                    ppu.current_scanline[*pixels_drawn as usize] = palette::convert_bg_to_absolute_palette(pal_1);
                    ppu.current_scanline[*pixels_drawn as usize + 1] = palette::convert_bg_to_absolute_palette(pal_2);
                    *pixels_drawn += 2;
                }
            }
        } else {
            for two_pixels in data.into_iter().copied() {
                if *pixels_to_skip > 0 {
                    if *pixels_to_skip == 1 {
                        // We should skip the first pixel in the pair
                        *pixels_to_skip = 0;
                        let palette_index = palette_base + two_pixels.get_bits(4, 7);
                        ppu.current_scanline[*pixels_drawn as usize] =
                            palette::convert_bg_to_absolute_palette(palette_index);
                        *pixels_drawn += 1;
                    } else {
                        *pixels_to_skip -= 2;
                    }
                } else {
                    // Since we're drawing 2 pixels at a time we need to check if we're near the max
                    if *pixels_drawn >= (DISPLAY_WIDTH - 1) as i16 {
                        if *pixels_drawn == DISPLAY_WIDTH as i16 {
                            break;
                        } else {
                            // Draw one last pixel
                            //TODO: Verify if 0..=3 or 4..=7
                            let palette_index = palette_base + two_pixels.get_bits(0, 3);
                            ppu.current_scanline[*pixels_drawn as usize] =
                                palette::convert_bg_to_absolute_palette(palette_index);
                            *pixels_drawn += 1;
                            break;
                        }
                    }

                    let (pal_1, pal_2) = (
                        palette_base + two_pixels.get_bits(0, 3),
                        palette_base + two_pixels.get_bits(4, 7),
                    );

                    ppu.current_scanline[*pixels_drawn as usize] = palette::convert_bg_to_absolute_palette(pal_1);
                    ppu.current_scanline[*pixels_drawn as usize + 1] = palette::convert_bg_to_absolute_palette(pal_2);
                    *pixels_drawn += 2;
                }
            }
        }
    };
}
