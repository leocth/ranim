use clap::Parser;
use color_eyre::Result;
use ranim_render::args::Args;

fn main() -> Result<()> {
    pollster::block_on(run())
}

async fn run() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let args = Args::parse();

    if args.preview {
        ranim_render::preview().await
    } else {
        ranim_render::output(args).await
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::prelude::*;

//     fn derivative_salvation() -> Scene {
//         let mut scn = Scene::new();

//         let ax = Axes::new()
//             .x_range([-2, 11, 1])
//             .y_range([-5, 100, 10])
//             .with_tips()
//             .with_coordinates()
//             .done();

//         let func = |x: f32| x.powi(2);
//         let x_squared = ax.plot(func).color(BLUE).done();
//         let x_squared_tex =
//             MathTex::new(r"\int 2x~\mathrm{d}x = x^2").move_to(x_square.get_right() + 4.3 * LEFT + UP);

//         scn.play(Create::from([ax, x_squared, x_squared_tex]));
//         // alternative syntax
//         // play!(scn, Create, [ax, x_squared, x_squared_tex]);
//         scn.wait();

//         scn
//     }

// }
