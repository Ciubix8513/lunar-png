#![allow(clippy::cast_precision_loss)]
use std::{cell::OnceCell, io::Read, num::NonZeroU64, path::PathBuf, str::FromStr, sync::OnceLock};

use bytemuck::{bytes_of, Zeroable};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt, StagingBelt},
    BindGroupEntry, BufferUsages, CommandEncoderDescriptor, Extent3d, Features, TextureDescriptor,
    TextureUsages,
};
use winit::{
    application::ApplicationHandler,
    window::{Window, WindowAttributes},
};

struct App<'a> {
    device: OnceCell<wgpu::Device>,
    first_resume: bool,
    format: OnceCell<wgpu::TextureFormat>,
    images: Vec<(lunar_png::Image, String)>,
    pipeline: OnceCell<wgpu::RenderPipeline>,
    queue: OnceCell<wgpu::Queue>,
    size: (u32, u32),
    surface: OnceCell<wgpu::Surface<'a>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    img_data_buf: OnceCell<wgpu::Buffer>,
    img_data_binding: OnceCell<wgpu::BindGroup>,
    belt: OnceCell<StagingBelt>,
    img_index: usize,
    img_bindgroups: Vec<ImageBind>,
}

struct ImageBind {
    bindgroup: wgpu::BindGroup,
    aspect: f32,
}

#[repr(C)]
#[derive(bytemuck::Pod, Zeroable, Clone, Copy)]
struct ImgData {
    resolution: [f32; 2],
    aspect: f32,
    padding: f32,
}

static WINDOW: OnceLock<Window> = OnceLock::new();

impl App<'_> {
    fn new() -> Self {
        Self {
            images: Vec::new(),
            first_resume: true,
            size: (0, 0),
            surface: OnceCell::new(),
            queue: OnceCell::new(),
            device: OnceCell::new(),
            format: OnceCell::new(),
            pipeline: OnceCell::new(),
            img_data_buf: OnceCell::new(),
            surface_config: None,
            img_data_binding: OnceCell::new(),
            belt: OnceCell::new(),
            img_index: 0,
            img_bindgroups: Vec::new(),
        }
    }
}

impl ApplicationHandler for App<'_> {
    #[allow(clippy::too_many_lines)]
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.first_resume {
            return;
        }

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        self.first_resume = false;

        let win = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        win.set_visible(true);

        self.size = win.inner_size().into();

        WINDOW.set(win).unwrap();

        let win = WINDOW.get().unwrap();

        let instace = wgpu::Instance::default();

        let surface = instace.create_surface(win).unwrap();
        let adapter = futures::executor::block_on(req_adapter(
            instace,
            &wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            },
        ))
        .unwrap();

        let can_load_16 = !(adapter.features() & Features::TEXTURE_FORMAT_16BIT_NORM).is_empty();

        let e = futures::executor::block_on(req_device(
            &adapter,
            &wgpu::DeviceDescriptor {
                required_features: if can_load_16 {
                    Features::TEXTURE_FORMAT_16BIT_NORM
                } else {
                    Features::default()
                },
                ..Default::default()
            },
        ));

        if e.is_err() {
            let err = e.as_ref().unwrap_err();

            println!("{err:?}");
        }

        let (device, queue): (wgpu::Device, wgpu::Queue) = e.unwrap();
        self.device.set(device).unwrap();

        let device = self.device.get().unwrap();

        let format = surface
            .get_capabilities(&adapter)
            .formats
            .last()
            .copied()
            .unwrap();
        self.format.set(format).unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: self.size.0,
            height: self.size.1,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format],
        };

        self.surface_config = Some(config);

        let b_layout_0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("meow"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let b_layout_1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        for i in &mut self.images {
            println!("{}", i.1);
            let i = &mut i.0;
            i.add_channels();
            i.add_alpha();

            let texture = device.create_texture_with_data(
                &queue,
                &TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        height: i.height,
                        width: i.width,
                        ..Default::default()
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: match i.img_type {
                        lunar_png::ImageType::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
                        lunar_png::ImageType::Rgba16 => wgpu::TextureFormat::Rgba16Unorm,
                        _ => unreachable!(),
                    },
                    usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[match i.img_type {
                        lunar_png::ImageType::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
                        lunar_png::ImageType::Rgba16 => wgpu::TextureFormat::Rgba16Unorm,
                        _ => unreachable!(),
                    }],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &i.data,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(match i.img_type {
                    lunar_png::ImageType::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
                    lunar_png::ImageType::Rgba16 => wgpu::TextureFormat::Rgba16Unorm,
                    _ => unreachable!(),
                }),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: None,
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &b_layout_1,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            self.img_bindgroups.push(ImageBind {
                aspect: i.width as f32 / i.height as f32,
                bindgroup: bind_group,
            });
        }

        self.queue.set(queue).unwrap();

        let p_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("p"),
            bind_group_layouts: &[&b_layout_0, &b_layout_1],
            push_constant_ranges: &[],
        });

        let v = device.create_shader_module(include_wgsl!("./shaders/vert.wgsl"));
        let f = device.create_shader_module(include_wgsl!("./shaders/frag.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&p_layout),
            vertex: wgpu::VertexState {
                module: &v,
                entry_point: "main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &f,
                entry_point: "main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        let d = ImgData {
            resolution: [self.size.0 as f32, self.size.1 as f32],
            aspect: self.images[0].0.width as f32 / self.images[0].0.height as f32,
            padding: 0.0,
        };

        println!("Aspect = {}", d.aspect);

        let resolution_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&d),
        });

        let img_data_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &b_layout_0,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    resolution_buffer.as_entire_buffer_binding(),
                ),
            }],
        });

        let belt = StagingBelt::new(512);

        self.belt.set(belt).unwrap();
        self.img_data_binding.set(img_data_bindgroup).unwrap();
        self.img_data_buf.set(resolution_buffer).unwrap();
        self.pipeline.set(pipeline).unwrap();

        surface.configure(device, self.surface_config.as_ref().unwrap());
        self.surface.set(surface).unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if !event.state.is_pressed() {
                    return;
                }
                self.img_index = (self.img_index + 1) % self.images.len();
            }
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => {
                let config = self.surface_config.as_mut().unwrap();
                print!("Resized {:?} to ", self.size);
                self.size = (size.width, size.height);
                println!("{:?}", self.size);
                config.width = size.width;
                config.height = size.height;

                self.surface
                    .get()
                    .unwrap()
                    .configure(self.device.get().unwrap(), config);
            }
            winit::event::WindowEvent::RedrawRequested => {
                let color = self.surface.get().unwrap().get_current_texture().unwrap();

                let color_view = color.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: Some(*self.format.get().unwrap()),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });

                let mut encoder = self
                    .device
                    .get()
                    .unwrap()
                    .create_command_encoder(&CommandEncoderDescriptor { label: None });

                let belt = self.belt.get_mut().unwrap();

                let buf = self.img_data_buf.get().unwrap();
                let d = ImgData {
                    resolution: [self.size.0 as f32, self.size.1 as f32],
                    aspect: self.img_bindgroups[self.img_index].aspect,
                    padding: 0.0,
                };

                belt.write_buffer(
                    &mut encoder,
                    buf,
                    0,
                    NonZeroU64::new(16).unwrap(),
                    self.device.get().unwrap(),
                )
                .copy_from_slice(bytes_of(&d));

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 1.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(self.pipeline.get().unwrap());
                pass.set_bind_group(0, self.img_data_binding.get().unwrap(), &[]);
                pass.set_bind_group(1, &self.img_bindgroups[self.img_index].bindgroup, &[]);
                pass.draw(0..6, 0..1);
                drop(pass);

                let cmd = encoder.finish();
                belt.finish();
                self.queue.get().unwrap().submit(Some(cmd));
                belt.recall();

                color.present();
                WINDOW.get().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

#[allow(clippy::future_not_send)]
async fn req_adapter<'a, 'b>(
    instance: wgpu::Instance,
    options: &wgpu::RequestAdapterOptions<'a, 'b>,
) -> Option<wgpu::Adapter> {
    instance.request_adapter(options).await
}

#[allow(clippy::future_not_send)]
async fn req_device<'a>(
    adapter: &wgpu::Adapter,
    descriptor: &wgpu::DeviceDescriptor<'a>,
) -> Result<(wgpu::Device, wgpu::Queue), wgpu::RequestDeviceError> {
    adapter.request_device(descriptor, None).await
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = App::new();

    let mut args = std::env::args();

    //skip the first one
    args.next();

    let mut images = Vec::new();

    for a in args {
        let p = PathBuf::from_str(&a).unwrap();

        if !p.exists() {
            println!("File {a} doesn't exist!");
            return;
        }

        if p.is_dir() {
            continue;
        }

        let mut f = std::fs::File::open(p).unwrap();
        let mut d = Vec::new();

        f.read_to_end(&mut d).unwrap();

        let data1 = lunar_png::decode_png(&mut d.into_iter()).unwrap();
        images.push((data1, a));
    }

    if images.is_empty() {
        println!("no images");
        return;
    }

    app.images = images;

    event_loop.run_app(&mut app).unwrap();
}
