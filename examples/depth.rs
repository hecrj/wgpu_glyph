use wgpu_glyph::{GlyphBrushBuilder, Scale, Section};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

fn main() -> Result<(), String> {
    env_logger::init();

    // Initialize GPU
    let adapter: wgpu::Adapter = futures::executor::block_on(wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
        },
        wgpu::BackendBit::all(),
    )).expect("Request adapter");

    let (mut device, queue) = futures::executor::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits { max_bind_groups: 1 },
        }));

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let surface = wgpu::Surface::create(&window);

    // Prepare swap chain and depth buffer
    let mut size = window.inner_size();
    let mut new_size = None;

    let (mut swap_chain, mut depth_view) =
        create_frame_views(&mut device, &surface, size);

    // Prepare glyph_brush
    let inconsolata: &[u8] = include_bytes!("Inconsolata-Regular.ttf");

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(inconsolata)
        .expect("Load fonts")
        .depth_stencil_state(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        })
        .build(&device, FORMAT);

    // Render loop
    window.request_redraw();

    event_loop.run(move |event, _, control_flow| {
        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(size),
                ..
            } => {
                new_size = Some(size);
            }
            winit::event::Event::RedrawRequested { .. } => {
                if let Some(new_size) = new_size.take() {
                    let (new_swap_chain, new_depth_view) =
                        create_frame_views(&mut device, &surface, new_size);

                    swap_chain = new_swap_chain;
                    depth_view = new_depth_view;
                    size = new_size;
                }

                // Get a command encoder for the current frame
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                );

                // Get the next frame
                let frame = match swap_chain.get_next_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        return;
                    }
                };

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

                // Queue text on top, it will be drawn first.
                // Depth buffer will make it appear on top.
                glyph_brush.queue(Section {
                    screen_position: (30.0, 30.0),
                    text: "On top",
                    scale: Scale::uniform(95.0),
                    color: [0.8, 0.8, 0.8, 1.0],
                    z: 0.9,
                    ..Section::default()
                });

                // Queue background text next.
                // Without a depth buffer, this text would be rendered on top of the
                // previous queued text.
                glyph_brush.queue(Section {
                    bounds: (size.width as f32, size.height as f32),
                    text: &include_str!("lipsum.txt")
                        .replace("\n\n", "")
                        .repeat(10),
                    scale: Scale::uniform(30.0),
                    color: [0.05, 0.05, 0.1, 1.0],
                    z: 0.2,
                    ..Section::default()
                });

                // Draw all the text!
                glyph_brush
                    .draw_queued(
                        &mut device,
                        &mut encoder,
                        &frame.view,
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &depth_view,
                            depth_load_op: wgpu::LoadOp::Clear,
                            depth_store_op: wgpu::StoreOp::Store,
                            stencil_load_op: wgpu::LoadOp::Clear,
                            stencil_store_op: wgpu::StoreOp::Store,
                            clear_depth: -1.0,
                            clear_stencil: 0,
                        },
                        size.width,
                        size.height,
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

fn create_frame_views(
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    size: winit::dpi::PhysicalSize<u32>,
) -> (wgpu::SwapChain, wgpu::TextureView) {
    let (width, height) = (size.width, size.height);

    let swap_chain = device.create_swap_chain(
        surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: FORMAT,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
        },
    );

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    });

    (swap_chain, depth_texture.create_default_view())
}
