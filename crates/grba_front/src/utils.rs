use grba_core::emulator::ppu::RGBA;
use grba_core::FRAMEBUFFER_SIZE;
use image::imageops::FilterType;
use image::ImageBuffer;
use std::path::{Path, PathBuf};

pub struct MainArgs {
    pub execute_path: Option<PathBuf>,
    pub start_paused: bool,
    pub bios: PathBuf,
    pub start_bios: bool,
}

pub fn parse_main_args() -> Option<MainArgs> {
    let mut parser = pico_args::Arguments::from_env();

    Some(MainArgs {
        start_paused: parser.contains(["-p", "--paused"]),
        start_bios: parser.contains(["-s", "--start-bios"]),
        bios: parser
            .opt_value_from_str("--bios")
            .ok()?
            .unwrap_or("roms/gba_bios.bin".into()),
        execute_path: parser.opt_free_from_str().ok()?,
    })
}

pub trait BoolUtils {
    fn toggle(&mut self);
}

impl BoolUtils for bool {
    fn toggle(&mut self) {
        *self = !*self;
    }
}

pub fn save_rgba_image(framebuffer: &[RGBA; FRAMEBUFFER_SIZE], path: impl AsRef<Path>) {
    let frame_array: &[u8; grba_core::FRAMEBUFFER_SIZE * std::mem::size_of::<RGBA>()] =
        unsafe { std::mem::transmute(&framebuffer) };

    let temp_buffer: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(240, 160, frame_array.to_vec()).unwrap();
    let temp_buffer = image::imageops::resize(&temp_buffer, 320, 288, FilterType::Nearest);
    temp_buffer.save(path).unwrap();
}
