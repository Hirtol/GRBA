use parsing::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Region {
    Japan,
    Europe,
    French,
    Spanish,
    Usa,
    German,
    Italian,
}

/// Represents the Cartridge Header for a GBA rom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeHeader {
    /// Uppercase ASCII, max `12` characters
    game_title: String,
    /// Uppercase ASCII, max `4` characters
    game_code: String,
    /// Uppercase ASCII, max `2` characters
    maker_code: String,
    /// (00h for current GBA models)
    main_unit_code: u8,
    /// (usually 00h) (bit7=DACS/debug related)
    device_type: u8,
    /// (usually 00h)
    software_version: u8,
    /// Header checksum (We'll probably just ignore this one)
    complement_checksum: u8,
}

impl CartridgeHeader {
    /// Create a new [CartridgeHeader].
    ///
    /// # Arguments
    ///
    /// * `rom` - The ROM data to parse. Should be the full file's binary contents.
    pub fn new(rom: &[u8]) -> Self {
        let (calculated_chksum, read_chksum) = (Self::calculate_checksum(rom), parse_complement_checksum(rom));

        if calculated_chksum != read_chksum {
            log::warn!(
                "Checksum mismatch! Calculated: {}, Read: {}, continuing Cartridge load...",
                calculated_chksum,
                read_chksum
            );
        }

        Self {
            game_title: parse_title(rom),
            game_code: parse_game_code(rom),
            maker_code: parse_maker_code(rom),
            main_unit_code: parse_main_unit_code(rom),
            device_type: parse_device_type(rom),
            software_version: parse_software_version(rom),
            complement_checksum: read_chksum,
        }
    }

    pub fn region(&self) -> Option<Region> {
        parse_region(&self.game_code)
    }

    fn calculate_checksum(rom: &[u8]) -> u8 {
        let checksum = rom[0xA0..0xBC].iter().fold(0u8, |acc, &i| acc.wrapping_sub(i));

        checksum.wrapping_sub(0x19)
    }
}

mod parsing {
    use crate::cartridge::header::Region;

    pub fn parse_title(rom: &[u8]) -> String {
        String::from_utf8_lossy(&rom[0xA0..0xAC])
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn parse_game_code(rom: &[u8]) -> String {
        String::from_utf8_lossy(&rom[0xAC..0xB0])
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn parse_region(game_code: &str) -> Option<Region> {
        match game_code.chars().nth(3)? {
            'J' => Region::Japan,
            'P' => Region::Europe,
            'F' => Region::French,
            'S' => Region::Spanish,
            'E' => Region::Usa,
            'D' => Region::German,
            'I' => Region::Italian,
            _ => {
                log::info!("Unknown region code: {}, defaulting to `Japan`", game_code);
                Region::Japan
            }
        }
        .into()
    }

    pub fn parse_maker_code(rom: &[u8]) -> String {
        String::from_utf8_lossy(&rom[0xB0..0xB2])
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn parse_main_unit_code(rom: &[u8]) -> u8 {
        rom[0xB3]
    }

    pub fn parse_device_type(rom: &[u8]) -> u8 {
        rom[0xB4]
    }

    pub fn parse_software_version(rom: &[u8]) -> u8 {
        rom[0xBC]
    }

    pub fn parse_complement_checksum(rom: &[u8]) -> u8 {
        rom[0xBD]
    }
}
