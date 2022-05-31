use std::{fmt::Display, path::PathBuf, str::FromStr};

use clap::Parser;

use crate::util::Size;

/// Renderer frontend of `ranim`
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Toggles preview mode. If set to true, animations will be displayed on a new preview window
    /// instead of an image or video file.
    #[clap(short, long)]
    pub preview: bool,

    /// The quality of the output image or video in output mode.
    ///
    /// Possible quality options include: High (h/high) for 1920x1080, 60fps;
    /// Medium (m/medium) for 1280x720, 30fps; Low (l/low) for 864x480, 15fps.
    #[clap(short, long, default_value_t = Quality::Low)]
    pub quality: Quality,

    #[clap(short, long, default_value = "media/output")]
    pub output_file: PathBuf,

    #[clap(long)]
    pub single_frame: bool,

    #[clap(long)]
    pub no_output: bool
}

#[derive(Debug, Clone, Copy)]
pub enum Quality {
    FourK,
    Production,
    High,
    Medium,
    Low,
}
impl Quality {
    pub fn size(self) -> Size {
        match self {
            Quality::FourK => Size::new(3840, 2160),
            Quality::Production => Size::new(2560, 1440),
            Quality::High => Size::new(1920, 1080),
            Quality::Medium => Size::new(1280, 720),
            Quality::Low => Size::new(854, 480),
        }
    }
    pub fn frame_rate(self) -> u32 {
        match self {
            Quality::High | Quality::Production | Quality::FourK => 60,
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
            "high" | "hi" | "h" => Ok(Self::High),
            "medium" | "mid"| "m" => Ok(Self::Medium),
            "low" | "lo" | "l" => Ok(Self::Low),
            "production" | "prod" | "p" => Ok(Self::Production),
            "fourk" | "4k" | "k" => Ok(Self::FourK),
            _ => Err(format!("Invalid quality: {s}")),
        }
    }
}
