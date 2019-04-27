mod cache;

pub use cache::Cache;

use std::mem;
use std::rc::Rc;

use glyph_brush::rusttype::{point, Rect};

pub struct Pipeline {
    transform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    cache: Rc<Cache>,
    uniform_layout: wgpu::BindGroupLayout,
    uniforms: wgpu::BindGroup,
    instances: wgpu::Buffer,
}

impl Pipeline {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub const IDENTITY_MATRIX: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    pub fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        cache_width: u32,
        cache_height: u32,
    ) -> Pipeline {
        let transform = device
            .create_buffer_mapped(
                16,
                wgpu::BufferUsageFlags::UNIFORM
                    | wgpu::BufferUsageFlags::TRANSFER_DST,
            )
            .fill_from_slice(&Self::IDENTITY_MATRIX);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            r_address_mode: wgpu::AddressMode::ClampToEdge,
            s_address_mode: wgpu::AddressMode::ClampToEdge,
            t_address_mode: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode,
            min_filter: filter_mode,
            mipmap_filter: filter_mode,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            max_anisotropy: 0,
            compare_function: wgpu::CompareFunction::Always,
            border_color: wgpu::BorderColor::TransparentBlack,
        });

        let cache = Cache::new(device, cache_width, cache_height);

        let uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStageFlags::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer,
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStageFlags::FRAGMENT,
                        ty: wgpu::BindingType::Sampler,
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 2,
                        visibility: wgpu::ShaderStageFlags::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture,
                    },
                ],
            });

        let uniforms = Self::create_uniforms(
            device,
            &uniform_layout,
            &transform,
            &sampler,
            &cache.view,
        );

        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            size: mem::size_of::<Instance>() as u32 * Instance::MAX as u32,
            usage: wgpu::BufferUsageFlags::VERTEX
                | wgpu::BufferUsageFlags::TRANSFER_DST,
        });

        Pipeline {
            transform,
            sampler,
            cache: Rc::new(cache),
            uniform_layout,
            uniforms,
            instances,
        }
    }

    pub fn cache(&self) -> Rc<Cache> {
        self.cache.clone()
    }

    pub fn increase_cache_size(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) {
        self.cache = Rc::new(Cache::new(device, width, height));

        self.uniforms = Self::create_uniforms(
            device,
            &self.uniform_layout,
            &self.transform,
            &self.sampler,
            &self.cache.view,
        );
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        transform: [f32; 16],
        instances: &[Instance],
    ) {
    }

    pub fn redraw(&self, encoder: &mut wgpu::CommandEncoder) {}

    // Helpers
    fn create_uniforms(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        transform: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        cache: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: transform,
                        range: 0..64,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(cache),
                },
            ],
        })
    }
}

#[derive(Clone)]
pub struct Instance {
    left_top: [f32; 3],
    right_bottom: [f32; 2],
    tex_left_top: [f32; 2],
    tex_right_bottom: [f32; 2],
    color: [f32; 4],
}

impl Instance {
    const MAX: usize = 50_000;
}

impl From<glyph_brush::GlyphVertex> for Instance {
    #[inline]
    fn from(vertex: glyph_brush::GlyphVertex) -> Instance {
        let glyph_brush::GlyphVertex {
            mut tex_coords,
            pixel_coords,
            bounds,
            screen_dimensions: (screen_w, screen_h),
            color,
            z,
        } = vertex;

        let gl_bounds = Rect {
            min: point(
                2.0 * (bounds.min.x / screen_w - 0.5),
                2.0 * (bounds.min.y / screen_h - 0.5),
            ),
            max: point(
                2.0 * (bounds.max.x / screen_w - 0.5),
                2.0 * (bounds.max.y / screen_h - 0.5),
            ),
        };

        let mut gl_rect = Rect {
            min: point(
                2.0 * (pixel_coords.min.x as f32 / screen_w - 0.5),
                2.0 * (pixel_coords.min.y as f32 / screen_h - 0.5),
            ),
            max: point(
                2.0 * (pixel_coords.max.x as f32 / screen_w - 0.5),
                2.0 * (pixel_coords.max.y as f32 / screen_h - 0.5),
            ),
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
            left_top: [gl_rect.min.x, gl_rect.max.y, z],
            right_bottom: [gl_rect.max.x, gl_rect.min.y],
            tex_left_top: [tex_coords.min.x, tex_coords.max.y],
            tex_right_bottom: [tex_coords.max.x, tex_coords.min.y],
            color,
        }
    }
}
