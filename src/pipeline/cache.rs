use wgpu::util::DeviceExt;

pub struct Cache {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
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
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
            mip_level_count: 1,
            sample_count: 1,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

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
        let width = size[0] as usize;
        let height = size[1] as usize;

        // It is a webgpu requirement that:
        //  BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
        // So we calculate padded_width by rounding width
        // up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_width_padding = (align - width % align) % align;
        let padded_width = width + padded_width_padding;

        let mut padded_data = vec![0; padded_width * height];
        for row in 0..height {
            padded_data[row * padded_width..row * padded_width + width]
                .copy_from_slice(&data[row * width..(row + 1) * width])
        }
        let buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &padded_data,
                usage: wgpu::BufferUsage::COPY_SRC,
            });

        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &buffer,
                layout: wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: padded_width as u32,
                    rows_per_image: height as u32,
                },
            },
            wgpu::TextureCopyView {
                texture: &self.texture,
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
