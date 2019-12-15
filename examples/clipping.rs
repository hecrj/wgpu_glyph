use wgpu_glyph::{GlyphBrushBuilder, Region, Scale, Section};

fn main() -> Result<(), String> {
    env_logger::init();

    // Initialize GPU
    let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        backends: wgpu::BackendBit::all(),
    })
    .expect("Request adapter");

    let (mut device, mut queue) =
        adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits { max_bind_groups: 1 },
        });

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let surface = wgpu::Surface::create(&window);

    // Prepare swap chain
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let mut size = window.inner_size().to_physical(window.hidpi_factor());

    let mut swap_chain = device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: render_format,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        },
    );

    // Prepare glyph_brush
    let inconsolata: &[u8] = include_bytes!("Inconsolata-Regular.ttf");
    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(inconsolata)
        .build(&device, render_format);

    // Render loop
    window.request_redraw();

    event_loop.run(move |event, _, control_flow| {
        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size.to_physical(window.hidpi_factor());

                swap_chain = device.create_swap_chain(
                    &surface,
                    &wgpu::SwapChainDescriptor {
                        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                        format: render_format,
                        width: size.width.round() as u32,
                        height: size.height.round() as u32,
                        present_mode: wgpu::PresentMode::Vsync,
                    },
                );
            }
            winit::event::Event::EventsCleared => {
                // Get a command encoder for the current frame
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { todo: 0 },
                );

                // Get the next frame
                let frame = swap_chain.get_next_texture();

                // Clear frame
                {
                    let _ = encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            color_attachments: &[
                                wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.view,
                                    resolve_target: None,
                                    load_op: wgpu::LoadOp::Clear,
                                    store_op: wgpu::StoreOp::Store,
                                    clear_color: wgpu::Color {
                                        r: 0.4,
                                        g: 0.4,
                                        b: 0.4,
                                        a: 1.0,
                                    },
                                },
                            ],
                            depth_stencil_attachment: None,
                        },
                    );
                }

                glyph_brush.queue(Section {
                    text: "Hello wgpu_glyph!",
                    screen_position: (30.0, 30.0),
                    color: [0.0, 0.0, 0.0, 1.0],
                    scale: Scale { x: 40.0, y: 40.0 },
                    bounds: (size.width as f32, size.height as f32),
                    ..Section::default()
                });

                // Draw the text!
                glyph_brush
                    .draw_queued(
                        &mut device,
                        &mut encoder,
                        &frame.view,
                        size.width.round() as u32,
                        size.height.round() as u32,
                    )
                    .expect("Draw queued");

                glyph_brush.queue(Section {
                    text: "Hello wgpu_glyph!",
                    screen_position: (30.0, 90.0),
                    color: [1.0, 1.0, 1.0, 1.0],
                    scale: Scale { x: 40.0, y: 40.0 },
                    bounds: (size.width as f32, size.height as f32),
                    ..Section::default()
                });

                // Draw the text!
                glyph_brush
                    .draw_queued_with_transform_and_scissoring(
                        &mut device,
                        &mut encoder,
                        &frame.view,
                        orthographic_projection(
                            size.width.round() as u32,
                            size.height.round() as u32,
                        ),
                        Region {
                            x: 40,
                            y: 105,
                            width: 200,
                            height: 15,
                        },
                    )
                    .expect("Draw queued");

                queue.submit(&[encoder.finish()]);
            }
            _ => {
                *control_flow = winit::event_loop::ControlFlow::Wait;
            }
        }
    })
}

fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, 2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, -1.0, 0.0, 1.0,
    ]
}
