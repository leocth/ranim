use std::{
    ffi::CString,
    ops::{Div, Index, IndexMut, Mul},
    path::PathBuf,
    simd::{f32x4, u8x4},
};

use crate::canvas::{CanvasBufferView, CanvasSize};
use color_eyre::{eyre::eyre, Result};
use cstr::cstr;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avformat::AVFormatContextOutput,
    avutil::{ra, AVDictionary, AVFrame},
    error::RsmpegError,
};

use super::{OutputBehavior, PIXEL_STRIDE};

pub struct VideoOutput {
    encode_context: AVCodecContext,
    frame: AVFrame,
    output_format_context: AVFormatContextOutput,

    size: CanvasSize,
    frame_cnt: i64,
}
impl VideoOutput {
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
                self.output_format_context
                    .streams()
                    .get(0)
                    .unwrap()
                    .time_base,
            );
            self.output_format_context.write_frame(&mut packet)?;
        }
        Ok(())
    }
}
impl VideoOutput {
    pub fn new(size: CanvasSize, mut output_path: PathBuf) -> Result<Self> {
        if output_path.extension().is_none() {
            output_path.set_extension("mp4");
        }

        let encode_context = {
            let encoder = AVCodec::find_encoder_by_name(cstr!("libx264"))
                .ok_or_else(|| eyre!("Failed to find encoder codec"))?;
            let mut ctx = AVCodecContext::new(&encoder);
            ctx.set_bit_rate(2500 * 1000); // 2500 kbps
            ctx.set_width(size.size.width as i32);
            ctx.set_height(size.size.height as i32);
            ctx.set_time_base(ra(1, 60));
            ctx.set_framerate(ra(60, 1));
            ctx.set_gop_size(10);
            ctx.set_max_b_frames(1);
            ctx.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
            let dict = AVDictionary::from_string(cstr!("profile=high"), cstr!("="), cstr!(";"), 0)
                .expect("Failed to parse dictionary string");
            ctx.open(Some(dict))?;
            ctx
        };

        let mut frame = AVFrame::new();
        frame.set_format(encode_context.pix_fmt);
        frame.set_width(encode_context.width);
        frame.set_height(encode_context.height);
        frame.alloc_buffer()?;

        let output_format_context = {
            let output_path = CString::new(output_path.to_string_lossy().as_ref()).unwrap();
            let mut output_format_context = AVFormatContextOutput::create(&output_path, None)?;
            {
                let mut stream = output_format_context.new_stream();
                // autodetect output format based on filename
                stream.set_codecpar(encode_context.extract_codecpar());
                stream.set_time_base(encode_context.time_base);
            }
            output_format_context.dump(0, &output_path)?;
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
}

impl OutputBehavior for VideoOutput {
    fn encode_frame<'bv>(&mut self, buf: &CanvasBufferView<'bv>) -> Result<()> {
        let width = self.size.size.width as usize;
        let height = self.size.size.height as usize;
        let mut dst_y = FrameData::new(&self.frame, 0, height);
        let mut dst_u = FrameData::new(&self.frame, 1, height / 2);
        let mut dst_v = FrameData::new(&self.frame, 2, height / 2);

        rgba2yuv420(
            &buf.view,
            &mut dst_y,
            &mut dst_u,
            &mut dst_v,
            self.size.bytes_per_row as usize,
            width,
            height,
        );

        self.frame.set_pts(self.frame_cnt);
        self.frame_cnt += 1;

        self.encode_context.send_frame(Some(&self.frame))?;
        self.write()
    }

    fn conclude(&mut self) -> Result<()> {
        self.encode_context.send_frame(None)?;
        self.write()?;
        self.output_format_context.write_trailer()?;
        Ok(())
    }
}

struct FrameData<'a> {
    buf: &'a mut [u8],
    linesize: usize,
}
impl<'a> FrameData<'a> {
    fn new(frame: &AVFrame, index: usize, height: usize) -> Self {
        let data = frame.data[index];
        let linesize = frame.linesize[index] as usize;
        let buf = unsafe { std::slice::from_raw_parts_mut(data, linesize * height) };
        Self { buf, linesize }
    }
}
impl<'a> Index<(usize, usize)> for FrameData<'a> {
    type Output = u8;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.buf[y * self.linesize + x]
    }
}
impl<'a> IndexMut<(usize, usize)> for FrameData<'a> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.buf[y * self.linesize + x]
    }
}

//              src buffer (RGBA)                               dst buffer (YUV)
//
//        |<       width        >|                        |<       width        >|
//       -+----------------------+- -+                    +----------------------+- -+
//       ^|                      |   |           |\       |                      |   |
//        |       RGBA data       fil       =====| \      |       YUV data        fil
// height |       stride=4       |ler|      |       \     |                      |ler|
//        |                                 |       /     |
//       V|                      |   |      =====| /      |                      |   |
//       -+----------------------+- -+           |/       +----------------------+- -+
//        |<      padded_width      >|                    |<        linesize        >|

fn rgba2yuv420(
    src: &[u8],
    dst_y: &mut FrameData<'_>,
    dst_u: &mut FrameData<'_>,
    dst_v: &mut FrameData<'_>,
    src_bytes_per_row: usize,
    width: usize,
    height: usize,
) {
    let y_vec = f32x4::from([0.2578125, 0.50390625, 0.09765625, 0.0]);
    let u_vec = f32x4::from([-0.1484375, -0.2890625, 0.4375, 0.0]);
    let v_vec = f32x4::from([0.4375, -0.3671875, -0.0703125, 0.0]);

    let pixel = |x: usize, y: usize| -> f32x4 {
        let base_pos = x * PIXEL_STRIDE as usize + y * src_bytes_per_row;
        u8x4::from_slice(&src[base_pos..base_pos + PIXEL_STRIDE as usize]).cast()
    };
    let mut write_y = |x: usize, y: usize, rgba: f32x4| {
        dst_y[(x, y)] = (rgba.mul(y_vec).reduce_sum() + 16.0) as u8;
    };
    let mut write_u = |x: usize, y: usize, rgba: f32x4| {
        dst_u[(x, y)] = (rgba.mul(u_vec).reduce_sum() + 128.0) as u8;
    };
    let mut write_v = |x: usize, y: usize, rgba: f32x4| {
        dst_v[(x, y)] = (rgba.mul(v_vec).reduce_sum() + 128.0) as u8;
    };

    for y in 0..height / 2 {
        for x in 0..width / 2 {
            let px = x * 2;
            let py = y * 2;

            let pix00 = pixel(px, py);
            let pix01 = pixel(px, py + 1);
            let pix10 = pixel(px + 1, py);
            let pix11 = pixel(px + 1, py + 1);

            let fours = f32x4::from([4.0, 4.0, 4.0, 4.0]);
            let avg_pix = (pix00 + pix01 + pix10 + pix11).div(fours);

            write_y(px, py, pix00);
            write_y(px, py + 1, pix01);
            write_y(px + 1, py, pix10);
            write_y(px + 1, py + 1, pix11);
            write_u(x, y, avg_pix);
            write_v(x, y, avg_pix);
        }
    }
}
