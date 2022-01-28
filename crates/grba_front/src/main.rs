use crate::rendering::{Renderer, RendererOptions};
use egui_wgpu_backend::wgpu::{PresentMode, TextureFormat};
use grba_core::cartridge::{Cartridge, CARTRIDGE_SRAM_START};
use log::LevelFilter;
use pixels::wgpu::{Backends, PowerPreference, RequestAdapterOptions};
use pixels::{wgpu, Pixels, PixelsBuilder, SurfaceTexture};
use std::fs::read;
use std::thread;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

mod rendering;

fn main() {
    let cfg = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("grba_front")
        .build();

    simplelog::SimpleLogger::init(LevelFilter::Trace, cfg).unwrap();

    let event_loop = EventLoop::new();
    let mut input = winit_input_helper::WinitInputHelper::new();

    let mut renderer = Renderer::new(&event_loop, RendererOptions::default()).unwrap();
    let mut now = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        println!("Event: {:?} - {:#?}", event, now.elapsed());
        now = Instant::now();
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            // Update renderer state and request new frame.
            renderer.after_window_update(&input);
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                renderer.framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                let render_result = renderer.render_pixels(&[255; grba_core::FRAMEBUFFER_SIZE]);

                // Basic error handling
                if render_result.is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });

    let mut mm = memmap2::MmapOptions::new();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./testing_ram.bin")
        .unwrap();
    file.set_len(grba_core::cartridge::CARTRIDGE_RAM_SIZE as u64).unwrap();
    let mut map = unsafe { mm.map_mut(&file).unwrap() };
    let mut cart = Cartridge::new(
        &read("C:\\Users\\Valentijn\\Desktop\\Rust\\GBA-Project\\grba\\roms\\Kirby_nightmare.gba").unwrap(),
        Box::new(map),
    );

    for (i, val) in b"Hello WOsrld".iter().enumerate() {
        cart.write_sram(CARTRIDGE_SRAM_START + i as u32, *val);
    }
}
