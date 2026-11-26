//! The page handed to a render's encoder on the GPU (docs/blitz-bevy.feature B1d): the page's picture, alone or laid
//! over a 3D target, written into the packed RGBA8 buffer raea-render's fed render composites and encodes. Nothing comes
//! back to the CPU here; the encoder's NV12 is the render's one copy.
//!
//! Each texture is copied as stored (sRGB-encoded bytes) into a buffer the pass reads, so the bytes are the ones a
//! readback gave; the pass does on the GPU what raea did on the CPU (motion.rs bevy_over_3d), integer for
//! integer, so a render's frames are the same.

const WGSL: &str = r"
struct P { w: u32, h: u32, mode: u32, _pad: u32 }
@group(0) @binding(0) var page: texture_2d<f32>;
@group(0) @binding(1) var under: texture_2d<f32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;
@group(0) @binding(3) var<uniform> p: P;
fn pack(c: vec4f) -> u32 {
    let u = vec4u(clamp(round(c * 255.0), vec4f(0.0), vec4f(255.0)));
    return u.r | (u.g << 8u) | (u.b << 16u) | (u.a << 24u);
}
fn ch(v: u32, i: u32) -> u32 { return (v >> (8u * i)) & 255u; }
@compute @workgroup_size(16, 16) fn main(@builtin(global_invocation_id) g: vec3u) {
    if g.x >= p.w || g.y >= p.h { return; }
    let xy = vec2i(g.xy);
    let s = pack(textureLoad(page, xy, 0));
    var o = s;
    if p.mode == 2u {
        // the page alone, opaque: over the black the encoder's buffer starts as
        o = s | (255u << 24u);
    } else if p.mode == 1u {
        // the page over the 3D, the 3D (premultiplied, as Bevy blends into it) over black: its own colour, a window's
        // picture, whatever alpha its fragments wrote; then Skia's SrcOver on the u8 grid, as bevy_over_3d does
        let d = pack(textureLoad(under, xy, 0));
        let k = 256u - ch(s, 3u);
        o = 255u << 24u;
        for (var i = 0u; i < 3u; i++) {
            let t = ch(d, i);
            o |= min(255u, ch(s, i) + ((t * k) >> 8u)) << (8u * i);
        }
    }
    out[g.y * p.w + g.x] = o;
}
";

/// View `tex` as linear `Rgba8Unorm`/`Bgra8Unorm` so compute shaders read raw sRGB bytes without conversion.
pub fn unorm_view(tex: &wgpu::Texture) -> wgpu::TextureView {
    let format = match tex.format() {
        wgpu::TextureFormat::Rgba8UnormSrgb => Some(wgpu::TextureFormat::Rgba8Unorm),
        wgpu::TextureFormat::Bgra8UnormSrgb => Some(wgpu::TextureFormat::Bgra8Unorm),
        _ => None,
    };
    tex.create_view(&wgpu::TextureViewDescriptor { format, ..Default::default() })
}

/// The pass and its persistent uniform and dummy texture: textures are sampled directly with no staging copy.
pub struct Feed {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    _dummy_tex: wgpu::Texture,
    dummy: wgpu::TextureView,
}

impl Feed {
    pub fn new(device: &wgpu::Device) -> Feed {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("raea-page feed"), source: wgpu::ShaderSource::Wgsl(WGSL.into()) });
        let tex = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raea-page feed"),
            entries: &[
                tex(0),
                tex(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("raea-page feed"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raea-page feed"),
            layout: Some(&pl),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor { label: Some("raea-page feed"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let dummy_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raea-page feed dummy"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy = dummy_tex.create_view(&Default::default());
        Feed { pipeline, layout, uniform, _dummy_tex: dummy_tex, dummy }
    }

    /// The top left `w` x `h` of `page` (as stored), over the top left of `under` when given, else made `opaque` or left
    /// as it is, into `out` (packed RGBA8, `w` x `h`); submitted on `queue`, after what was submitted before it.
    #[allow(clippy::too_many_arguments)]
    pub fn run(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, page: &wgpu::Texture, under: Option<&wgpu::Texture>, opaque: bool, w: u32, h: u32, out: &wgpu::Buffer) {
        let page_view = unorm_view(page);
        let under_holder = under.map(unorm_view);
        let under_view = under_holder.as_ref().unwrap_or(&self.dummy);
        let mode: u32 = if under.is_some() { 1 } else if opaque { 2 } else { 0 };
        let mut params = [0u8; 16];
        params[0..4].copy_from_slice(&w.to_le_bytes());
        params[4..8].copy_from_slice(&h.to_le_bytes());
        params[8..12].copy_from_slice(&mode.to_le_bytes());
        queue.write_buffer(&self.uniform, 0, &params);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raea-page feed"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&page_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(under_view) },
                wgpu::BindGroupEntry { binding: 2, resource: out.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: self.uniform.as_entire_binding() },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("raea-page feed") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("raea-page feed"), timestamp_writes: None });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(16), h.div_ceil(16), 1);
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgsl_parses() {
        wgpu::naga::front::wgsl::parse_str(WGSL).unwrap();
    }

    #[test]
    fn feed_texture_pass_matches_cpu_src_over() {
        let Ok((_, _, device, queue)) = raea_gpu_device::device() else { return };
        let (w, h) = (8u32, 4u32);
        let page_px: Vec<u8> = (0..(w * h * 4) as usize).map(|i| ((i * 37 + 13) % 256) as u8).collect();
        let under_px: Vec<u8> = (0..(w * h * 4) as usize).map(|i| ((i * 53 + 7) % 256) as u8).collect();
        let make_tex = |px: &[u8]| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
            });
            queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, px, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None }, wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
            t
        };
        let page = make_tex(&page_px);
        let under = make_tex(&under_px);
        let size = (w * h * 4) as u64;
        let out = device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let read = device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut feed = Feed::new(&device);
        let read_out = |feed: &mut Feed, u: Option<&wgpu::Texture>, opaque: bool| -> Vec<u8> {
            feed.run(&device, &queue, &page, u, opaque, w, h, &out);
            let mut enc = device.create_command_encoder(&Default::default());
            enc.copy_buffer_to_buffer(&out, 0, &read, 0, size);
            let idx = queue.submit([enc.finish()]);
            read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
            let got = read.slice(..).get_mapped_range().unwrap().to_vec();
            read.unmap();
            got
        };
        assert_eq!(read_out(&mut feed, None, false), page_px);
        let exp_opaque: Vec<u8> = page_px.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2], 255]).collect();
        assert_eq!(read_out(&mut feed, None, true), exp_opaque);
        let mut exp_over = under_px.clone();
        for (d, s) in exp_over.chunks_exact_mut(4).zip(page_px.chunks_exact(4)) {
            let k = 256 - s[3] as u32;
            for c in 0..3 {
                d[c] = (s[c] as u32 + ((d[c] as u32 * k) >> 8)).min(255) as u8;
            }
            d[3] = 255;
        }
        assert_eq!(read_out(&mut feed, Some(&under), true), exp_over);
    }
}
