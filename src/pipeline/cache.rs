use std::num::NonZeroU32;

pub struct Cache {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    padded_cache: Vec<u8>
}

impl Cache {
    const INITIAL_UPLOAD_BUFFER_SIZE: u64 =
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64 * 100;

    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Cache {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu_glyph::Cache"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            mip_level_count: 1,
            sample_count: 1,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut padded_cache = Vec::new();
        padded_cache.reserve(Self::INITIAL_UPLOAD_BUFFER_SIZE as usize);

        Cache {
            texture,
            view,
            padded_cache
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[u8],
    ) {
        let width = size[0] as usize;
        let height = size[1] as usize;

        // It is a webgpu requirement that:
        //  BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
        // So we calculate padded_width by rounding width
        // up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_width_padding = (align - width % align) % align;
        let padded_width = width + padded_width_padding;

        let padded_data_size = (padded_width * height) as u64;
        self.padded_cache.resize(padded_data_size as usize, 0);

        let padded_data = self.padded_cache.as_mut_slice();

        for row in 0..height {
            padded_data[row * padded_width..row * padded_width + width]
                .copy_from_slice(&data[row * width..(row + 1) * width]);
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: u32::from(offset[0]),
                    y: u32::from(offset[1]),
                    z: 0,
                },
            },
            &padded_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(padded_width as u32),
                rows_per_image: NonZeroU32::new(height as u32),
            },
            wgpu::Extent3d {
                width: size[0] as u32,
                height: size[1] as u32,
                depth_or_array_layers: 1,
            }
        );
    }
}
