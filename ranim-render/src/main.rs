use clap::Parser;
use color_eyre::Result;
use ranim::prelude::Scene;
use ranim_render::args::Args;

fn main() -> Result<()> {
    let scene = Scene::new();
    

    pollster::block_on(run())
}

async fn run() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let args = Args::parse();

    if args.preview {
        ranim_render::preview(args).await
    } else {
        ranim_render::output(args).await
    }
}
