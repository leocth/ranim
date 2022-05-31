use std::{
    ffi::CString,
    num::NonZeroU32,
    ops::{Index, IndexMut},
    simd::{f32x4, u8x4},
};

use color_eyre::Result;
use cstr::cstr;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avformat::AVFormatContextOutput,
    avutil::{ra, AVDictionary, AVFrame},
    error::RsmpegError,
};

use crate::{
    args::Args,
    data::RenderData,
    util::{Size, PIXEL_STRIDE},
    RenderPass, Renderer, RgbTexture, TextureAndView,
};

pub struct VideoRenderer {
    renderer: Renderer,
    pub data: RenderData,
    enc: VideoEncoder,
    rgb_texture: RgbTexture,
    yuv_texture: YuvTexture,
    yuv_buffer: YuvBuffer,
    render_pass: RenderPass,
    yuv_pass: YuvPass,
}
impl VideoRenderer {
    pub async fn new(args: Args) -> Result<Self> {
        let renderer = Renderer::new(&args).await?;
        let data = RenderData::new(&renderer);
        let enc = VideoEncoder::new(&args)?;
        let rgb_texture = RgbTexture::new(&renderer);
        let yuv_texture = YuvTexture::new(&renderer);
        let yuv_buffer = YuvBuffer::new(&renderer);
        let render_pass = RenderPass::new(&renderer, &data);
        let yuv_pass = YuvPass::new(&renderer, &rgb_texture);

        Ok(Self {
            renderer,
            data,
            enc,
            rgb_texture,
            yuv_texture,
            yuv_buffer,
            render_pass,
            yuv_pass,
        })
    }
    pub fn update(&mut self) {
        self.data.update(&self.renderer);
    }

    pub async fn render(&mut self) -> Result<()> {
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
        self.render_pass.execute(&mut encoder, &self.rgb_texture, &self.data);

        // encoder.copy_texture_to_buffer(
        //     wgpu::ImageCopyTexture {
        //         aspect: wgpu::TextureAspect::All,
        //         texture: &self.rgb_texture.tv.texture,
        //         mip_level: 0,
        //         origin: wgpu::Origin3d::ZERO,
        //     },
        //     wgpu::ImageCopyBuffer {
        //         buffer: &self.yuv_buffer.buf,
        //         layout: wgpu::ImageDataLayout {
        //             offset: 0,
        //             bytes_per_row: NonZeroU32::new(self.renderer.size.bytes_per_row),
        //             rows_per_image: NonZeroU32::new(self.renderer.size.height),
        //         },
        //     },
        //     self.renderer.size.extent(),
        // );

        self.yuv_pass
            .execute(&mut encoder, &self.rgb_texture, &self.yuv_buffer);
        self.renderer.queue.submit([encoder.finish()]);

        let view = self.yuv_buffer.view(&self.renderer.device).await;
        self.enc.encode(&view)?;
        self.yuv_buffer.unmap(view);

        Ok(())
    }

    pub fn conclude(&mut self) -> Result<()> {
        self.enc.conclude()
    }
}

pub struct YuvTexture {
    tv: TextureAndView,
}
impl YuvTexture {
    pub fn new(renderer: &Renderer) -> Self {
        let desc = wgpu::TextureDescriptor {
            size: renderer.size.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
            label: Some("YUV texture"),
        };
        let tv = TextureAndView::new(&renderer.device, &desc);
        Self { tv }
    }
}

pub struct YuvBuffer {
    buf: wgpu::Buffer,
}
impl YuvBuffer {
    pub fn new(renderer: &Renderer) -> Self {
        let desc = wgpu::BufferDescriptor {
            size: renderer.size.buffer_size() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: Some("YUV buffer"),
            mapped_at_creation: false,
        };
        let buf = renderer.device.create_buffer(&desc);
        Self { buf }
    }
    pub async fn view(&self, device: &wgpu::Device) -> YuvBufferView<'_> {
        let buf = self.buf.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        let mapping = buf.map_async(wgpu::MapMode::Read);
        device.poll(wgpu::Maintain::Wait);

        mapping
            .await
            .expect("Could not asynchronously map buffer to host");

        YuvBufferView {
            view: buf.get_mapped_range(),
        }
    }
    pub fn unmap(&self, view: YuvBufferView<'_>) {
        drop(view); // kill it
        self.buf.unmap();
    }
}

pub struct YuvBufferView<'a> {
    view: wgpu::BufferView<'a>,
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

pub struct VideoEncoder {
    encode_ctx: AVCodecContext,
    frame: AVFrame,
    output_ctx: AVFormatContextOutput,

    size: Size,
    frame_cnt: i64,
}
impl VideoEncoder {
    pub fn new(args: &Args) -> Result<Self> {
        let size = args.quality.size();
        let frame_rate = args.quality.frame_rate();
        let mut output_file = args.output_file.clone();

        if output_file.extension().is_none() {
            output_file.set_extension("mp4");
        }

        let encode_ctx = {
            let encoder = AVCodec::find_encoder_by_name(cstr!("h264_nvenc"))
                .or_else(|| AVCodec::find_encoder_by_name(cstr!("libx264")))
                .expect("Failed to find encoder codec");
            let mut ctx = AVCodecContext::new(&encoder);
            ctx.set_width(size.width as i32);
            ctx.set_height(size.height as i32);
            ctx.set_time_base(ra(1, frame_rate as i32));
            ctx.set_framerate(ra(frame_rate as i32, 1));
            ctx.set_gop_size(10);
            ctx.set_max_b_frames(1);
            ctx.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
            let dict = AVDictionary::from_string(
                cstr!("crf=28,profile=high,preset=fast"),
                cstr!("="),
                cstr!(","),
                0,
            )
            .expect("Failed to parse dictionary string");
            ctx.open(Some(dict))?;
            ctx
        };

        let mut frame = AVFrame::new();
        frame.set_format(encode_ctx.pix_fmt);
        frame.set_width(encode_ctx.width);
        frame.set_height(encode_ctx.height);
        frame.alloc_buffer()?;

        let output_ctx = {
            let output_path = CString::new(output_file.to_string_lossy().as_ref()).unwrap();
            let mut output_ctx = AVFormatContextOutput::create(&output_path, None)?;
            {
                let mut stream = output_ctx.new_stream();
                // autodetect output format based on filename
                stream.set_codecpar(encode_ctx.extract_codecpar());
                stream.set_time_base(encode_ctx.time_base);
            }
            output_ctx.dump(0, &output_path)?;
            output_ctx.write_header()?;
            output_ctx
        };

        Ok(Self {
            encode_ctx,
            frame,
            output_ctx,
            size,
            frame_cnt: 0,
        })
    }

    pub fn encode(&mut self, buf: &YuvBufferView<'_>) -> Result<()> {
        let width = self.size.width as usize;
        let height = self.size.height as usize;
        let mut dst_y = FrameData::new(&self.frame, 0, height);
        let mut dst_u = FrameData::new(&self.frame, 1, height / 2);
        let mut dst_v = FrameData::new(&self.frame, 2, height / 2);

        // println!("{:?}", &buf.view[0..16]);

        Self::rgba2yuv420(
            &buf.view,
            &mut dst_y,
            &mut dst_u,
            &mut dst_v,
            self.size.bytes_per_row as usize,
            width,
            height,
        );
        // println!("{:?}", &dst_y.buf[0..16]);
        // println!("{:?}", &dst_u.buf[0..8]);
        // println!("{:?}", &dst_v.buf[0..8]);
        // println!();

        self.frame.set_pts(self.frame_cnt);
        self.frame_cnt += 1;

        self.encode_ctx.send_frame(Some(&self.frame))?;
        self.write()
    }

    fn conclude(&mut self) -> Result<()> {
        self.encode_ctx.send_frame(None)?;
        self.write()?;
        self.output_ctx.write_trailer()?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        loop {
            let mut packet = match self.encode_ctx.receive_packet() {
                Ok(packet) => packet,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break
                }
                Err(e) => return Err(e.into()),
            };
            packet.rescale_ts(
                self.encode_ctx.time_base,
                self.output_ctx.streams().get(0).unwrap().time_base,
            );
            self.output_ctx.write_frame(&mut packet)?;
        }
        Ok(())
    }

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
            dst_y[(x, y)] = ((rgba * y_vec).reduce_sum() + 16.0) as u8;
        };
        let mut write_u = |x: usize, y: usize, rgba: f32x4| {
            dst_u[(x, y)] = ((rgba * u_vec).reduce_sum() + 128.0) as u8;
        };
        let mut write_v = |x: usize, y: usize, rgba: f32x4| {
            dst_v[(x, y)] = ((rgba * v_vec).reduce_sum() + 128.0) as u8;
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
                let avg_pix = (pix00 + pix01 + pix10 + pix11) / fours;

                write_y(px, py, pix00);
                write_y(px, py + 1, pix01);
                write_y(px + 1, py, pix10);
                write_y(px + 1, py + 1, pix11);
                write_u(x, y, avg_pix);
                write_v(x, y, avg_pix);
            }
        }
    }
}

pub struct YuvPass {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    dispatch_x: u32,
    dispatch_y: u32,
    size: Size,
}
impl YuvPass {
    pub fn new(renderer: &Renderer, rgb: &RgbTexture) -> Self {
        let shader = renderer
            .device
            .create_shader_module(&wgpu::include_wgsl!("shaders/yuv420.wgsl"));
        let pipeline = renderer
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("YUV pipeline"),
                layout: None,
                module: &shader,
                entry_point: "yuv_main",
            });
        let yuv = YuvTexture::new(renderer);
        let (dispatch_x, dispatch_y) = compute_work_group_count(renderer.size, (16, 16));

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Texture bind group"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&rgb.tv.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&yuv.tv.view),
                    },
                ],
            });

        Self {
            pipeline,
            bind_group,
            dispatch_x,
            dispatch_y,
            size: renderer.size,
        }
    }
    pub fn execute(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        // dst: &YuvTexture,
        dst: &RgbTexture,
        buf: &YuvBuffer,
    ) {
        // {
        //     let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        //         label: Some("YUV pass"),
        //     });
        //     pass.set_pipeline(&self.pipeline);
        //     pass.set_bind_group(0, &self.bind_group, &[]);
        //     pass.dispatch(self.dispatch_x, self.dispatch_y, 1);
        // }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &dst.tv.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buf.buf,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(self.size.bytes_per_row),
                    rows_per_image: NonZeroU32::new(self.size.height),
                },
            },
            self.size.extent(),
        );
    }
}

fn compute_work_group_count(
    size: Size,
    (workgroup_width, workgroup_height): (u32, u32),
) -> (u32, u32) {
    let x = (size.bytes_per_row / PIXEL_STRIDE as u32 + workgroup_width - 1) / workgroup_width;
    let y = (size.height + workgroup_height - 1) / workgroup_height;

    (x, y)
}
