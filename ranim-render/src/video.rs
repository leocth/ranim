use std::ffi::CStr;

use color_eyre::{eyre::eyre, Result};
use cstr::cstr;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avformat::AVFormatContextOutput,
    avutil::{ra, AVFrame}, error::RsmpegError,
};
use winit::dpi::PhysicalSize;

pub struct Encoder {
    encode_context: AVCodecContext,
    frame: AVFrame,
    output_format_context: AVFormatContextOutput,

    size: PhysicalSize<u32>,
    frame_cnt: i64,
}
impl Encoder {
    pub fn new(size: PhysicalSize<u32>, output_path: &CStr) -> Result<Self> {
        let encode_context = {
            let encoder = AVCodec::find_encoder_by_name(cstr!("png"))
                .ok_or_else(|| eyre!("Failed to find encoder codec"))?;
            let mut encode_context = AVCodecContext::new(&encoder);
            encode_context.set_bit_rate(400000);
            encode_context.set_width(size.width as i32);
            encode_context.set_height(size.height as i32);
            encode_context.set_time_base(ra(1, 60));
            encode_context.set_framerate(ra(60, 1));
            encode_context.set_gop_size(10);
            encode_context.set_max_b_frames(1);
            encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
            encode_context.open(None)?;
            encode_context
        };

        let mut frame = AVFrame::new();
        frame.set_format(encode_context.pix_fmt);
        frame.set_width(encode_context.width);
        frame.set_height(encode_context.height);
        frame.alloc_buffer()?;

        let output_format_context = {
            let mut output_format_context = AVFormatContextOutput::create(output_path, None)?;
            {
                let mut stream = output_format_context.new_stream();
                stream.set_codecpar(encode_context.extract_codecpar());
                stream.set_time_base(encode_context.time_base);
            }
            output_format_context.dump(0, output_path)?;
            output_format_context.write_header()?;
            output_format_context
        };
        Ok(Self {
            encode_context,
            frame,
            output_format_context,
            size,
            frame_cnt: 0,
        })
    }

    pub fn write_frame(&mut self, buffer: &[u8]) {
        let data = self.frame.data[0];
        let linesize = self.frame.linesize[0] as usize;
        let width = self.size.width as usize;
        let height = self.size.height as usize;
        let rgb_data = unsafe { std::slice::from_raw_parts_mut(data, height * linesize * 3) };


        for y in 0..height {
            for x in 0..width {
                let rgb_start = y * linesize + x * 3;
                let buffer_start = (y * width + x) * 4;
                // we're converting from RGBA to RGB data here
                rgb_data[rgb_start..=rgb_start + 2].copy_from_slice(
                    &buffer[buffer_start..=buffer_start + 2]
                );
            }
        }

        self.frame.set_pts(self.frame_cnt);

        self.frame_cnt += 1;
    }

    pub fn encode(&mut self) -> Result<()> {
        self.encode_context.send_frame(Some(&self.frame))?;
        self.write()
    }

    pub fn end(&mut self) -> Result<()> {
        self.encode_context.send_frame(None)?;
        self.write()?;
        self.output_format_context.write_trailer()?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        loop {
            let mut packet = match self.encode_context.receive_packet() {
                Ok(packet) => packet,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break
                }
                Err(e) => return Err(e.into()),
            };
            packet.rescale_ts(
                self.encode_context.time_base,
                self.output_format_context.streams().get(0).unwrap().time_base,
            );
            self.output_format_context.write_frame(&mut packet)?;
        }
        Ok(())
    }
}
