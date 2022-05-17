#![warn(clippy::pedantic)]
#![deny(rust_2018_idioms)]

use color_eyre::Result;
use renderer::{RenderMode, Renderer};
use winit::{
    dpi::PhysicalSize,
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod buf;
mod renderer;
pub mod video;

pub async fn preview() -> Result<()> {
    let mut input = WinitInputHelper::new();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = Renderer::new(RenderMode::Preview(&window)).await?;

    event_loop.run(move |event, _, control_flow| {
        if input.update(&event) {
            if input.key_released(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::Resized(physical_size) => renderer.resize(*physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(**new_inner_size)
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                renderer.update();
                // TODO: this is cringe
                let res = pollster::block_on(renderer.render());
                match res {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{e:?}"),
                }
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => {}
        }
    });
}

pub async fn output() -> Result<()> {
    let mut renderer = Renderer::new(RenderMode::Output {
        size: PhysicalSize {
            width: 854,
            height: 480,
        },
    })
    .await?;

    renderer.render().await?;

    Ok(())
}
