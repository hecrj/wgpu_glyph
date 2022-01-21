use std::error::Error;
use wgpu_glyph::{ab_glyph, GlyphBrushBuilder, Section, Text};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };

    // Initialize GPU
    let (device, mut queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Request device")
    });

    // Prepare swap chain and depth buffer
    let mut size = window.inner_size();
    let mut new_size = None;

    let mut depth_view = create_frame_views(&device, &surface, size);

    // Prepare glyph_brush
    let inconsolata = ab_glyph::FontArc::try_from_slice(include_bytes!(
        "Inconsolata-Regular.ttf"
    ))?;

    let mut glyph_brush = GlyphBrushBuilder::using_font(inconsolata)
        .depth_stencil_state(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
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
                    depth_view =
                        create_frame_views(&device, &surface, new_size);
                    size = new_size;
                }

                // Get a command encoder for the current frame
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("Redraw"),
                    },
                );

                // Get the next frame
                let frame =
                    surface.get_current_texture().expect("Get next frame");
                let view = &frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Clear frame
                {
                    let _ = encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some("Render pass"),
                            color_attachments: &[
                                wgpu::RenderPassColorAttachment {
                                    view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(
                                            wgpu::Color {
                                                r: 0.4,
                                                g: 0.4,
                                                b: 0.4,
                                                a: 1.0,
                                            },
                                        ),
                                        store: true,
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
                    text: vec![Text::default()
                        .with_text("On top")
                        .with_scale(95.0)
                        .with_color([0.8, 0.8, 0.8, 1.0])
                        .with_z(0.9)],
                    ..Section::default()
                });

                // Queue background text next.
                // Without a depth buffer, this text would be rendered on top of the
                // previous queued text.
                glyph_brush.queue(Section {
                    bounds: (size.width as f32, size.height as f32),
                    text: vec![Text::default()
                        .with_text(
                            &include_str!("lipsum.txt")
                                .replace("\n\n", "")
                                .repeat(10),
                        )
                        .with_scale(30.0)
                        .with_color([0.05, 0.05, 0.1, 1.0])
                        .with_z(0.2)],
                    ..Section::default()
                });

                // Draw all the text!
                glyph_brush
                    .draw_queued(
                        &device,
                        &mut queue,
                        &mut encoder,
                        view,
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(-1.0),
                                store: true,
                            }),
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: true,
                            }),
                        },
                        size.width,
                        size.height,
                    )
                    .expect("Draw queued");

                // Submit the work!
                queue.submit(Some(encoder.finish()));
                frame.present();
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
) -> wgpu::TextureView {
    let (width, height) = (size.width, size.height);

    surface.configure(
        device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: FORMAT,
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        },
    );

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth buffer"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    });

    depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
}
