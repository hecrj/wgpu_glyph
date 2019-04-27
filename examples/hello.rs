use wgpu::winit;
use wgpu_glyph::{GlyphBrushBuilder, Section};

fn main() -> Result<(), String> {
    // Initialize GPU
    let instance = wgpu::Instance::new();

    let adapter = instance.get_adapter(&wgpu::AdapterDescriptor {
        power_preference: wgpu::PowerPreference::HighPerformance,
    });

    let mut device = adapter.create_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
    });

    // Open window and create a surface
    let mut events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .with_resizable(false)
        .build(&events_loop)
        .unwrap();
    let surface = instance.create_surface(&window);

    // Prepare swap chain
    let size = window
        .get_inner_size()
        .unwrap()
        .to_physical(window.get_hidpi_factor());
    let mut swap_chain = device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsageFlags::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
        },
    );

    // Prepare glyph_brush
    let inconsolata: &[u8] = include_bytes!("Inconsolata-Regular.ttf");
    let mut glyph_brush =
        GlyphBrushBuilder::using_font_bytes(inconsolata).build(&mut device);

    // Render loop
    let mut running = true;

    while running {
        // Close window when requested
        events_loop.poll_events(|event| match event {
            winit::Event::WindowEvent {
                event: winit::WindowEvent::CloseRequested,
                ..
            } => running = false,
            _ => {}
        });

        // Get a command encoder for the current frame
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                todo: 0,
            });

        // Get the next frame
        let frame = swap_chain.get_next_texture();

        // Clear frame
        {
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    },
                ],
                depth_stencil_attachment: None,
            });
        }

        // Queue the text
        glyph_brush.queue(Section {
            text: "Hello wgpu_glyph",
            ..Section::default()
        });

        // Draw the text!
        glyph_brush.draw_queued(
            &mut device,
            &mut encoder,
            &frame.view,
            size.width.round() as u32,
            size.height.round() as u32,
        )?;
    }

    Ok(())
}
