mod cache;

use crate::Region;
use cache::Cache;

use bytemuck::{Pod, Zeroable};
use core::num::NonZeroU64;
use glyph_brush::ab_glyph::{point, Rect};
use std::marker::PhantomData;
use std::mem;

pub struct Pipeline<Depth> {
    transform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    cache: Cache,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: wgpu::BindGroup,
    raw: wgpu::RenderPipeline,
    instances: wgpu::Buffer,
    current_instances: usize,
    supported_instances: usize,
    current_transform: [f32; 16],
    depth: PhantomData<Depth>,
}

impl Pipeline<()> {
    pub fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        cache_width: u32,
        cache_height: u32,
    ) -> Pipeline<()> {
        build(
            device,
            filter_mode,
            render_format,
            None,
            cache_width,
            cache_height,
        )
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
        region: Option<Region>,
    ) {
        draw(
            self,
            device,
            staging_belt,
            encoder,
            target,
            None,
            transform,
            region,
        );
    }
}

impl Pipeline<wgpu::DepthStencilState> {
    pub fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        depth_stencil_state: wgpu::DepthStencilState,
        cache_width: u32,
        cache_height: u32,
    ) -> Pipeline<wgpu::DepthStencilState> {
        build(
            device,
            filter_mode,
            render_format,
            Some(depth_stencil_state),
            cache_width,
            cache_height,
        )
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachment,
        transform: [f32; 16],
        region: Option<Region>,
    ) {
        draw(
            self,
            device,
            staging_belt,
            encoder,
            target,
            Some(depth_stencil_attachment),
            transform,
            region,
        );
    }
}

impl<Depth> Pipeline<Depth> {
    pub fn update_cache(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[u8],
    ) {
        self.cache
            .update(device, staging_belt, encoder, offset, size, data);
    }

    pub fn increase_cache_size(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) {
        self.cache = Cache::new(device, width, height);

        self.uniforms = create_uniforms(
            device,
            &self.uniform_layout,
            &self.transform,
            &self.sampler,
            &self.cache.view,
        );
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        instances: &[Instance],
    ) {
        if instances.is_empty() {
            self.current_instances = 0;
            return;
        }

        if instances.len() > self.supported_instances {
            self.instances = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("wgpu_glyph::Pipeline instances"),
                size: mem::size_of::<Instance>() as u64
                    * instances.len() as u64,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });

            self.supported_instances = instances.len();
        }

        let instances_bytes = bytemuck::cast_slice(instances);

        if let Some(size) = NonZeroU64::new(instances_bytes.len() as u64) {
            let mut instances_view = staging_belt.write_buffer(
                encoder,
                &self.instances,
                0,
                size,
                device,
            );

            instances_view.copy_from_slice(instances_bytes);
        }

        self.current_instances = instances.len();
    }
}

// Helpers
#[cfg_attr(rustfmt, rustfmt_skip)]
const IDENTITY_MATRIX: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

fn build<D>(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    depth_stencil: Option<wgpu::DepthStencilState>,
    cache_width: u32,
    cache_height: u32,
) -> Pipeline<D> {
    use wgpu::util::DeviceExt;

    let transform =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&IDENTITY_MATRIX),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode,
        min_filter: filter_mode,
        mipmap_filter: filter_mode,
        ..Default::default()
    });

    let cache = Cache::new(device, cache_width, cache_height);

    let uniform_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wgpu_glyph::Pipeline uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<[f32; 16]>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: false,
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

    let uniforms = create_uniforms(
        device,
        &uniform_layout,
        &transform,
        &sampler,
        &cache.view,
    );

    let instances = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("wgpu_glyph::Pipeline instances"),
        size: mem::size_of::<Instance>() as u64
            * Instance::INITIAL_AMOUNT as u64,
        usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    let layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&uniform_layout],
        });

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Glyph Shader"),
        source: wgpu::ShaderSource::Wgsl(crate::Cow::Borrowed(include_str!(
            "shader/glyph.wgsl"
        ))),
        flags: wgpu::ShaderFlags::all(),
    });

    let raw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<Instance>() as u64,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 4 * 3,
                    },
                    wgpu::VertexAttribute {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 4 * (3 + 2),
                    },
                    wgpu::VertexAttribute {
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 4 * (3 + 2 + 2),
                    },
                    wgpu::VertexAttribute {
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 4 * (3 + 2 + 2 + 2),
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            front_face: wgpu::FrontFace::Cw,
            ..Default::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[wgpu::ColorTargetState {
                format: render_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrite::ALL,
            }],
        }),
    });

    Pipeline {
        transform,
        sampler,
        cache,
        uniform_layout,
        uniforms,
        raw,
        instances,
        current_instances: 0,
        supported_instances: Instance::INITIAL_AMOUNT,
        current_transform: [0.0; 16],
        depth: PhantomData,
    }
}

fn draw<D>(
    pipeline: &mut Pipeline<D>,
    device: &wgpu::Device,
    staging_belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment>,
    transform: [f32; 16],
    region: Option<Region>,
) {
    if transform != pipeline.current_transform {
        let mut transform_view = staging_belt.write_buffer(
            encoder,
            &pipeline.transform,
            0,
            unsafe { NonZeroU64::new_unchecked(16 * 4) },
            device,
        );

        transform_view.copy_from_slice(bytemuck::cast_slice(&transform));

        pipeline.current_transform = transform;
    }

    let mut render_pass =
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu_glyph::pipeline render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment,
        });

    render_pass.set_pipeline(&pipeline.raw);
    render_pass.set_bind_group(0, &pipeline.uniforms, &[]);
    render_pass.set_vertex_buffer(0, pipeline.instances.slice(..));

    if let Some(region) = region {
        render_pass.set_scissor_rect(
            region.x,
            region.y,
            region.width,
            region.height,
        );
    }

    render_pass.draw(0..4, 0..pipeline.current_instances as u32);
}

fn create_uniforms(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    transform: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    cache: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wgpu_glyph::Pipeline uniforms"),
        layout: layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: transform,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(cache),
            },
        ],
    })
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Instance {
    left_top: [f32; 3],
    right_bottom: [f32; 2],
    tex_left_top: [f32; 2],
    tex_right_bottom: [f32; 2],
    color: [f32; 4],
}

impl Instance {
    const INITIAL_AMOUNT: usize = 50_000;

    pub fn from_vertex(
        glyph_brush::GlyphVertex {
            mut tex_coords,
            pixel_coords,
            bounds,
            extra,
        }: glyph_brush::GlyphVertex,
    ) -> Instance {
        let gl_bounds = bounds;

        let mut gl_rect = Rect {
            min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
            max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
        };

        // handle overlapping bounds, modify uv_rect to preserve texture aspect
        if gl_rect.max.x > gl_bounds.max.x {
            let old_width = gl_rect.width();
            gl_rect.max.x = gl_bounds.max.x;
            tex_coords.max.x = tex_coords.min.x
                + tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.min.x < gl_bounds.min.x {
            let old_width = gl_rect.width();
            gl_rect.min.x = gl_bounds.min.x;
            tex_coords.min.x = tex_coords.max.x
                - tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.max.y > gl_bounds.max.y {
            let old_height = gl_rect.height();
            gl_rect.max.y = gl_bounds.max.y;
            tex_coords.max.y = tex_coords.min.y
                + tex_coords.height() * gl_rect.height() / old_height;
        }

        if gl_rect.min.y < gl_bounds.min.y {
            let old_height = gl_rect.height();
            gl_rect.min.y = gl_bounds.min.y;
            tex_coords.min.y = tex_coords.max.y
                - tex_coords.height() * gl_rect.height() / old_height;
        }

        Instance {
            left_top: [gl_rect.min.x, gl_rect.max.y, extra.z],
            right_bottom: [gl_rect.max.x, gl_rect.min.y],
            tex_left_top: [tex_coords.min.x, tex_coords.max.y],
            tex_right_bottom: [tex_coords.max.x, tex_coords.min.y],
            color: extra.color,
        }
    }
}
