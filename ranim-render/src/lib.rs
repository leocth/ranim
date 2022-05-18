#![warn(clippy::pedantic)]
#![deny(rust_2018_idioms)]

use std::{fmt::Display, str::FromStr};

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
mod video;

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

                if let Err(e) = res {
                    if let Some(e) = e.downcast_ref::<wgpu::SurfaceError>() {
                        match e {
                            // Reconfigure the surface if lost
                            wgpu::SurfaceError::Lost => renderer.resize(renderer.size),
                            // The system is out of memory, we should probably quit
                            wgpu::SurfaceError::OutOfMemory => *control_flow = ControlFlow::Exit,
                            _ => {}
                        }
                    }
                    eprintln!("{e:?}");
                }
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => {}
        }
    });
}

pub async fn output(quality: Quality) -> Result<()> {
    let mut renderer = Renderer::new(RenderMode::Output {
        size: quality.size(),
    })
    .await?;

    renderer.render().await?;

    renderer.finish()?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Quality {
    High,
    Medium,
    Low,
}
impl Quality {
    pub fn size(self) -> PhysicalSize<u32> {
        match self {
            Quality::High => PhysicalSize::new(1920, 1080),
            Quality::Medium => PhysicalSize::new(1280, 720),
            Quality::Low => PhysicalSize::new(854, 480),
        }
    }
    pub fn frame_rate(self) -> u32 {
        match self {
            Quality::High => 60,
            Quality::Medium => 30,
            Quality::Low => 15,
        }
    }
}
impl Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl FromStr for Quality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "high" | "h" => Ok(Self::High),
            "medium" | "m" => Ok(Self::Medium),
            "low" | "l" => Ok(Self::Low),
            _ => Err(format!("Invalid quality: {s}")),
        }
    }
}
