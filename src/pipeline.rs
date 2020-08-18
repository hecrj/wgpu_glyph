mod cache;

use crate::Region;
use cache::Cache;

use glyph_brush::ab_glyph::{point, Rect};
use std::marker::PhantomData;
use std::mem;
use zerocopy::AsBytes;
use wgpu::util::DeviceExt;

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
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
        region: Option<Region>,
    ) {
        draw(self, device, encoder, target, None, transform, region);
    }
}

impl Pipeline<wgpu::DepthStencilStateDescriptor> {
    pub fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        depth_stencil_state: wgpu::DepthStencilStateDescriptor,
        cache_width: u32,
        cache_height: u32,
    ) -> Pipeline<wgpu::DepthStencilStateDescriptor> {
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
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachmentDescriptor,
        transform: [f32; 16],
        region: Option<Region>,
    ) {
        draw(
            self,
            device,
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
        encoder: &mut wgpu::CommandEncoder,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[u8],
    ) {
        self.cache.update(device, encoder, offset, size, data);
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

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: instances.as_bytes(),
            usage: wgpu::BufferUsage::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(
            &instance_buffer,
            0,
            &self.instances,
            0,
            (mem::size_of::<Instance>() * instances.len()) as u64,
        );

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
    depth_stencil_state: Option<wgpu::DepthStencilStateDescriptor>,
    cache_width: u32,
    cache_height: u32,
) -> Pipeline<D> {
    let transform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: IDENTITY_MATRIX.as_bytes(),
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
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: wgpu::BufferSize::new(mem::size_of::<[f32; 16]>() as u64)
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        multisampled: false,
                    },
                    count: None
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

    let vs_module
        = device.create_shader_module(wgpu::include_spirv!("shader/vertex.spv"));

    let fs_module
        = device.create_shader_module(wgpu::include_spirv!("shader/fragment.spv"));

    let raw = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Cw,
            cull_mode: wgpu::CullMode::None,
            ..Default::default()
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip,
        color_states: &[wgpu::ColorStateDescriptor {
            format: render_format,
            color_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha_blend: wgpu::BlendDescriptor {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: mem::size_of::<Instance>() as u64,
                step_mode: wgpu::InputStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float3,
                        offset: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * 3,
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * (3 + 2),
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float2,
                        offset: 4 * (3 + 2 + 2),
                    },
                    wgpu::VertexAttributeDescriptor {
                        shader_location: 4,
                        format: wgpu::VertexFormat::Float4,
                        offset: 4 * (3 + 2 + 2 + 2),
                    },
                ],
            }],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
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
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    depth_stencil_attachment: Option<
        wgpu::RenderPassDepthStencilAttachmentDescriptor,
    >,
    transform: [f32; 16],
    region: Option<Region>,
) {
    if transform != pipeline.current_transform {
        let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: transform.as_bytes(),
            usage: wgpu::BufferUsage::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(
            &transform_buffer,
            0,
            &pipeline.transform,
            0,
            16 * 4,
        );

        pipeline.current_transform = transform;
    }

    let mut render_pass =
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: target,
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
                resource: wgpu::BindingResource::Buffer(transform.slice(..)),
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
#[derive(Debug, Clone, Copy, AsBytes)]
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
