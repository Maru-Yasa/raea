//! Packs an `Rgba8UnormSrgb` (with an `Rgba8Unorm` view format) or `Rgba8Unorm` GPU texture directly
//! into a storage buffer of packed RGBA8 `u32` words (`out[y * w + x]`), avoiding CPU staging readbacks.

const WGSL: &str = r"
struct P { w: u32, h: u32, _pad0: u32, _pad1: u32 }
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;
@group(0) @binding(2) var<uniform> p: P;
@compute @workgroup_size(16, 16) fn main(@builtin(global_invocation_id) g: vec3u) {
    if g.x >= p.w || g.y >= p.h { return; }
    let c = vec4u(clamp(round(textureLoad(src, vec2i(g.xy), 0) * 255.0), vec4f(0.0), vec4f(255.0)));
    out[g.y * p.w + g.x] = c.r | (c.g << 8u) | (c.b << 16u) | (c.a << 24u);
}
";

/// Reusable compute pass packing an `w` x `h` RGBA8 texture into a packed `u32` storage buffer on the GPU.
pub struct TexToBuf {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    w: u32,
    h: u32,
}

impl TexToBuf {
    pub fn new(device: &wgpu::Device, w: u32, h: u32) -> Self {
        use wgpu::util::DeviceExt;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raea tex-to-buf"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raea tex-to-buf"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("raea tex-to-buf"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raea tex-to-buf"),
            layout: Some(&pl),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut params = [0u8; 16];
        params[0..4].copy_from_slice(&w.to_le_bytes());
        params[4..8].copy_from_slice(&h.to_le_bytes());
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("raea tex-to-buf"),
            contents: &params,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        TexToBuf { pipeline, layout, uniform, w, h }
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        tex: &wgpu::Texture,
        out: &wgpu::Buffer,
        ts: Option<(&wgpu::QuerySet, u32, u32)>,
    ) {
        let view = raea_page::feed::unorm_view(tex);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raea tex-to-buf"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: out.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.uniform.as_entire_binding() },
            ],
        });
        let timestamp_writes = ts.map(|(query_set, a, b)| wgpu::ComputePassTimestampWrites {
            query_set,
            beginning_of_pass_write_index: Some(a),
            end_of_pass_write_index: Some(b),
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("raea tex-to-buf"),
            timestamp_writes,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(self.w.div_ceil(16), self.h.div_ceil(16), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_to_buf_wgsl_parses() {
        wgpu::naga::front::wgsl::parse_str(WGSL).unwrap();
    }

    #[test]
    fn tex_to_buf_preserves_srgb_bytes_exactly() {
        let Ok((_, _, device, queue)) = raea_gpu_device::device() else { return };
        let (w, h) = (16u32, 16u32);
        let px: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let size = (w * h * 4) as u64;
        let out = device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let read = device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let conv = TexToBuf::new(&device, w, h);
        let mut enc = device.create_command_encoder(&Default::default());
        conv.encode(&device, &mut enc, &tex, &out, None);
        enc.copy_buffer_to_buffer(&out, 0, &read, 0, size);
        let idx = queue.submit([enc.finish()]);
        read.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        let got = read.slice(..).get_mapped_range().unwrap().to_vec();
        read.unmap();
        assert_eq!(got, px);
    }
}
