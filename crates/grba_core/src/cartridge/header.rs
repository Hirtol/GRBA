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
    /// The game region
    region: Region,
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
        let (game_code, region) = parse_game_code(rom);
        Self {
            game_title: parse_title(rom),
            game_code,
            region,
            maker_code: parse_maker_code(rom),
            main_unit_code: parse_main_unit_code(rom),
            device_type: parse_device_type(rom),
            software_version: parse_software_version(rom),
            complement_checksum: parse_complement_checksum(rom),
        }
    }
}

mod parsing {
    use crate::cartridge::header::Region;

    pub fn parse_title(rom: &[u8]) -> String {
        String::from_utf8_lossy(&rom[0xA0..0xAC])
            .trim_matches(char::from(0))
            .to_string()
    }

    pub fn parse_game_code(rom: &[u8]) -> (String, Region) {
        let full_code = String::from_utf8_lossy(&rom[0xAC..0xB0])
            .trim_matches(char::from(0))
            .to_string();

        let region = parse_region(&full_code);
        (full_code, region)
    }

    fn parse_region(game_code: &str) -> Region {
        match game_code
            .chars()
            .nth(3)
            .expect("Game code should have at least 4 characters")
        {
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
