use clap::Parser;
use color_eyre::Result;
use ranim_render::Quality;

/// Renderer frontend of `ranim`
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Toggles preview mode. If set to true, animations will be displayed on a new preview window
    /// instead of an image or video file.
    #[clap(short, long)]
    preview: bool,

    /// The quality of the output image or video in output mode.
    /// 
    /// Possible quality options include: High (h/high) for 1920x1080, 60fps;
    /// Medium (m/medium) for 1280x720, 30fps; Low (l/low) for 864x480, 15fps.
    #[clap(short, long, default_value_t = Quality::Low)]
    quality: Quality,
}

fn main() -> Result<()> {
    pollster::block_on(run())
}

async fn run() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();
    
    let args = dbg!(Args::parse());

    let mut encode_context = {
        let encoder =
            AVCodec::find_encoder_by_name(cstr!("png")).context("Failed to find encoder codec")?;
        let mut encode_context = AVCodecContext::new(&encoder);
        encode_context.set_bit_rate(400000);
        encode_context.set_width(width);
        encode_context.set_height(height);
        encode_context.set_time_base(ra(1, 60));
        encode_context.set_framerate(ra(60, 1));
        encode_context.set_gop_size(10);
        encode_context.set_max_b_frames(1);
        encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
        encode_context.open(None)?;
        encode_context
    };


    if args.preview {
        ranim_render::preview().await
    } else {
        ranim_render::output(args.quality).await
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