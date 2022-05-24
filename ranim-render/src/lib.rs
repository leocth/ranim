#![feature(portable_simd)]
#![feature(array_chunks)]
#![deny(rust_2018_idioms)]

use args::Args;
use color_eyre::Result;
use data::types::Vertex;
use glam::{vec3, Vec4, vec4, Vec3};
use renderer::{RenderMode, Renderer};
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

use crate::data::types::Instance;

pub mod args;
mod camera;
mod canvas;
mod data;
mod output;
pub mod renderer;
mod util;

pub async fn preview(args: Args) -> Result<()> {
    let mut input = WinitInputHelper::new();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(args.quality.size())
        .build(&event_loop)?;
    let mut renderer = Renderer::new(RenderMode::Preview { window: &window }).await?;

    event_loop.run(move |event, _, control_flow| {
        if input.update(&event) && (input.key_released(VirtualKeyCode::Escape) || input.quit()) {
            *control_flow = ControlFlow::Exit;
            return;
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
                            wgpu::SurfaceError::Lost => renderer.reconfigure(),
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

pub async fn output(args: Args) -> Result<()> {
    use std::f32::consts::*;
    let mut renderer = Renderer::new(RenderMode::Output { args }).await?;

    renderer.data.vertices.push(Vertex {
        position: [0.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0],
    });

    let n = 40;
    for i in 1..=n {
        let rad = (i as f32 / n as f32) * TAU;
        renderer.data.vertices.extend([Vertex {
            position: [-rad.cos(), rad.sin(), 0.0],
            color: [1.0, 1.0, 1.0],
        }]);
        renderer.data.indices.extend([i, 0]);
    }
    renderer.data.indices.push(1);
    for _ in 0..60 {
        let x = 5.0 * rand::random::<f32>() - 2.5;
        let y = 5.0 * rand::random::<f32>() - 2.5;
        let s = 0.9 * rand::random::<f32>() + 0.1;
        let color: Vec3 = rand::random::<[f32; 3]>().into();

        renderer.data.instances.push(Instance {
            position: vec3(x, y, 0.0),
            scale: vec3(s, s, 0.0),
            color: vec4(color.x, color.y, color.z, 1.0),
            ..Instance::default()
        }.into());
        for _ in 0..2 {
            renderer.update();
            renderer.render().await?;
        }
    }


    // let t_end = fr * 3;

    // let mut inst_rot = 0.0;
    // let mut inst = Instance {
    //     position: vec3(-4.0, 0.0, 0.0),
    //     rotation: Quat::IDENTITY,
    //     scale: vec3(1.0, 1.0, 1.0),
    // };

    // for i in 0..t_end {
    //     let t = i as f32 / t_end as f32;

    //     if (i + 30) % (fr / 3) == 0 {
    //         // one second passed
    //         renderer.data.instances.push(inst);
    //         inst.position.x += 1.5;
    //         inst.position.y = t.sin();
    //         inst_rot += FRAC_PI_6;
    //         inst.rotation = Quat::from_rotation_z(inst_rot);
    //         inst.scale *= 0.8;
    //     }

    //     renderer.data.vertices[0].position[0] = 0.4 + 0.25 * (t * 5.0 * TAU).sin();
    //     renderer.data.vertices[1].position[0] = -0.4 + -0.25 * (t * 5.0 * TAU).sin();
    //     renderer.data.vertices[2].position[1] = 0.9 + 0.4 * (t * 5.0 * TAU).cos();

    //     // let translation = 0.6 * (t * 2.0 * TAU).sin();
    //     // let rotation = t * 0.7 * TAU;
    //     // renderer.data.camera.camera.position.y = translation;
    //     // renderer.data.camera.camera.rotation = rotation;

    //     renderer.update();
    //     renderer.render().await?;
    // }

    renderer.finish()?;
    Ok(())
}
