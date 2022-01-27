use grba_core::cartridge::{Cartridge, CARTRIDGE_SRAM_START};
use log::LevelFilter;
use pixels::{Pixels, SurfaceTexture};
use std::fs::read;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

mod gui;

fn main() {
    let cfg = simplelog::ConfigBuilder::new()
        .add_filter_allow_str("grba_front")
        .build();
    println!("Hey");
    simplelog::SimpleLogger::init(LevelFilter::Trace, cfg).unwrap();

    let event_loop = EventLoop::new();
    let mut input = winit_input_helper::WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("GRBA")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(grba_core::DISPLAY_WIDTH, grba_core::DISPLAY_HEIGHT, surface_texture).unwrap();
        let framework = gui::Framework::new(window_size.width, window_size.height, scale_factor, &pixels);

        (pixels, framework)
    };

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
                framework.resize(size.width, size.height);
            }

            // Update internal state and request a redraw
            window.request_redraw();
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the world
                // pixels.get_frame()

                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context)?;

                    Ok(())
                });

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
