use crate::emulator::MemoryAddress;
use crate::utils::BitOps;

pub const PALETTE_RAM_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct PaletteRam {
    /// The raw bytes used by the emulator for storage
    palette_ram: Box<[u8; PALETTE_RAM_SIZE]>,
    /// The persisted palette cache where RGB values are stored for quick lookup
    cache: Box<[Palette; 512]>,
}

impl PaletteRam {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a complete palette from the palette cache.
    #[inline(always)]
    pub fn get_palette(&self, index: usize) -> Palette {
        self.cache[index]
    }

    #[inline]
    pub const fn ram(&self) -> &[u8; PALETTE_RAM_SIZE] {
        &self.palette_ram
    }

    #[inline]
    pub const fn cache(&self) -> &[Palette; 512] {
        &self.cache
    }

    #[inline]
    pub fn read_palette(&self, address: MemoryAddress) -> u8 {
        let addr = address as usize % PALETTE_RAM_SIZE;

        self.palette_ram[addr]
    }

    #[inline]
    pub fn write_palette(&mut self, address: MemoryAddress, value: u8) {
        // When writing to palette ram with only a u8 the value is written to both the upper and lower bytes.
        let final_value = ((value as u16) << 8) | value as u16;

        self.write_palette_16(address, final_value);
    }

    #[inline]
    pub fn write_palette_16(&mut self, address: MemoryAddress, value: u16) {
        let addr = address as usize % PALETTE_RAM_SIZE;
        let data = value.to_le_bytes();
        // Better assembly
        assert!(addr < (PALETTE_RAM_SIZE - 1));

        self.palette_ram[addr] = data[0];
        self.palette_ram[addr + 1] = data[1];

        let cache_item = &mut self.cache[addr / 2];

        cache_item.red = convert_5_to_8_bit_color(value.get_bits(0, 4) as u8);
        cache_item.green = convert_5_to_8_bit_color(value.get_bits(5, 9) as u8);
        cache_item.blue = convert_5_to_8_bit_color(value.get_bits(10, 14) as u8);
    }
}

impl Default for PaletteRam {
    fn default() -> Self {
        Self {
            palette_ram: crate::box_array![0; PALETTE_RAM_SIZE],
            cache: crate::box_array![Palette::default(); 512],
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct RGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// 15 Bit RGB color pre-converted to 24 bit RGB
#[derive(Default, Debug, Copy, Clone)]
pub struct Palette {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Palette {
    pub const fn to_rgba(self, alpha: u8) -> RGBA {
        RGBA {
            red: self.red,
            green: self.green,
            blue: self.blue,
            alpha,
        }
    }
}

/// Convert the 5-bit colour values to 8 bit values which we work with.
///
/// Follows the method described [here](https://near.sh/articles/video/color-emulation)
/// TODO: Do the correction for washed out colours as well.
#[inline(always)]
pub const fn convert_5_to_8_bit_color(color_5: u8) -> u8 {
    let final_color = color_5 << 3;
    final_color | (color_5 >> 2)
}
