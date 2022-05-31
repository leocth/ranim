use clap::Parser;
use color_eyre::Result;
use glam::{vec4, vec3, Vec3};
use ranim_render::{args::Args, data::types::{Vertex, Instance}, video::VideoRenderer};

fn main() -> Result<()> {
    pollster::block_on(run())
}

async fn run() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let args = Args::parse();
    video(args).await?;

    Ok(())
}

async fn video(args: Args) -> Result<()> {
    use std::f32::consts::*;

    let mut renderer = VideoRenderer::new(args).await?;

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

        renderer.data.instances.push(
            Instance {
                position: vec3(x, y, 0.0),
                scale: vec3(s, s, 0.0),
                color: vec4(color.x, color.y, color.z, 1.0),
                ..Instance::default()
            }
            .into(),
        );
        for _ in 0..2 {
            renderer.update();
            renderer.render().await?;
        }
    }
    renderer.conclude()?;

    Ok(())
}
