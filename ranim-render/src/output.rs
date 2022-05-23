use color_eyre::Result;
use enum_dispatch::enum_dispatch;

use crate::canvas::CanvasBufferView;

use self::{image::ImageOutput, video::VideoOutput};

pub mod image;
pub mod video;

pub const PIXEL_STRIDE: u32 = 4;

#[enum_dispatch]
pub trait OutputBehavior: Sized {
    fn encode_frame<'bv>(&mut self, view: &CanvasBufferView<'bv>) -> Result<()>;
    fn conclude(&mut self) -> Result<()>;
}

#[enum_dispatch(OutputBehavior)]
pub enum Output {
    ImageOutput,
    VideoOutput,
}
