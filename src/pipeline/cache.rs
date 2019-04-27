pub struct Cache {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

impl Cache {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Cache {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            array_size: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsageFlags::TRANSFER_DST
                | wgpu::TextureUsageFlags::SAMPLED,
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
        let buffer = device
            .create_buffer_mapped(
                data.len(),
                wgpu::BufferUsageFlags::TRANSFER_SRC,
            )
            .fill_from_slice(data);

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &buffer,
                offset: 0,
                row_pitch: size[0] as u32,
                image_height: size[1] as u32,
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
                level: 0,
                slice: 0,
                origin: wgpu::Origin3d {
                    x: offset[0] as f32,
                    y: offset[1] as f32,
                    z: 0.0,
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
