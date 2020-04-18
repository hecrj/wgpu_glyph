pub struct Cache {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

fn multiple_of_256(value: usize) -> usize {
    if value % 256 == 0 {
        value
    } else {
        (value / 256 + 1) * 256
    }
}

impl Cache {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Cache {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu_glyph::Cache"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            array_layer_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            mip_level_count: 1,
            sample_count: 1,
        });

        let view = texture.create_default_view();

        Cache { texture, view }
    }

    pub fn update(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[u8],
    ) {
        let mut width = size[0] as usize;
        let height = size[1] as usize;
        let adjusted_width = multiple_of_256(height);

        let buffer = if width != adjusted_width {
            // Allocate a new vector with the correct padding
            let mut padded_data = vec![0; adjusted_width * height];
            
            // Copy into this vector
            for row in 0 .. height {
                padded_data[row * adjusted_width .. row * adjusted_width + width]
                    .copy_from_slice(&data[row * width .. (row + 1) * width]); 
            }

            device.create_buffer_with_data(&padded_data, wgpu::BufferUsage::COPY_SRC)
        } else {
            device.create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC)
        };

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &buffer,
                offset: 0,
                bytes_per_row: adjusted_width as u32,
                rows_per_image: size[1] as u32,
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
                array_layer: 0,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: u32::from(offset[0]),
                    y: u32::from(offset[1]),
                    z: 0,
                },
            },
            wgpu::Extent3d {
                width: size[0] as u32,
                height: size[1] as u32,
                depth: 1,
            },
        );
    }
}
