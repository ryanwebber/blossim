use std::time::{Duration, Instant};

use rand::Rng;
use wgpu::util::DeviceExt;
use winit::{event::WindowEvent, window::Window};

use crate::{
    gui, pipeline,
    storage::{self, Agent, Storable},
};

const APPROXIMATE_NUM_AGENTS: usize = 600000;

const QUAD_VERTICIES: &[storage::Vertex] = &[
    storage::Vertex {
        position: glam::f32::vec3(-1.0, 1.0, 0.0),
        uvs: glam::f32::vec2(0.0, 1.0),
    },
    storage::Vertex {
        position: glam::f32::vec3(-1.0, -1.0, 0.0),
        uvs: glam::f32::vec2(0.0, 0.0),
    },
    storage::Vertex {
        position: glam::f32::vec3(1.0, -1.0, 0.0),
        uvs: glam::f32::vec2(1.0, 0.0),
    },
    storage::Vertex {
        position: glam::f32::vec3(1.0, 1.0, 0.0),
        uvs: glam::f32::vec2(1.0, 1.0),
    },
];

const QUAD_INDICES: &[u32] = &[0, 1, 2, 2, 3, 0];

pub struct State {
    globals: Globals,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    pipelines: Pipelines,
    pipeline_data: PipelineData,
    gui_layer: GuiLayer,
}

pub struct Timing {
    pub time: Instant,
    pub time_since_last_frame: Duration,
    pub start_time: Instant,
    pub frame: usize,
}

pub struct Globals {
    pub timing: Timing,
    pub work_groups: glam::UVec3,
}

pub struct Pipelines {
    diffuse: pipeline::compute::ComputePipeline,
    simulation: pipeline::compute::ComputePipeline,
    render: pipeline::render::RenderPipeline,
}

pub struct PipelineData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    globals_buffer: wgpu::Buffer,
    agents_buffer: wgpu::Buffer,
    render_texture: wgpu::TextureView,
}

pub struct GuiLayer {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    interface: gui::Interface,
    enabled: bool,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let globals = Globals {
            timing: {
                let now = std::time::Instant::now();
                Timing {
                    time: now,
                    start_time: now,
                    time_since_last_frame: Duration::ZERO,
                    frame: 0,
                }
            },
            work_groups: {
                let x = (APPROXIMATE_NUM_AGENTS as f32).sqrt().ceil() as u32;
                glam::UVec3::new(x, x, 1)
            },
        };

        let num_work_groups = globals.work_groups.x * globals.work_groups.y * globals.work_groups.z;

        println!("Agents: {}, size: {}", num_work_groups, globals.work_groups);

        let agents: Vec<storage::Agent> = (0..num_work_groups)
            .map(|_| Agent {
                position: glam::f32::Vec2 {
                    x: rand::random::<f32>() * (size.width as f32),
                    y: rand::random::<f32>() * (size.height as f32),
                },
                velocity: random_unit_circle() * 32.0,
            })
            .collect();

        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code assumes an sRGB surface texture
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let pipelines = Pipelines {
            diffuse: pipeline::compute::ComputePipeline::diffuse(&device),
            simulation: pipeline::compute::ComputePipeline::simulation(&device),
            render: pipeline::render::RenderPipeline::new(&device, surface_format),
        };

        let pipeline_data = PipelineData {
            vertex_buffer: {
                let bytes = bytemuck::cast_slice(QUAD_VERTICIES);
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                })
            },
            index_buffer: {
                let bytes = bytemuck::cast_slice(QUAD_INDICES);
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::INDEX,
                })
            },
            globals_buffer: {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Globals buffer"),
                    contents: {
                        let storage: storage::Globals = (&globals).into();
                        let uniform = storage::Uniform(&storage);
                        &uniform.into_bytes()
                    },
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                })
            },
            agents_buffer: {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Agents buffer"),
                    contents: {
                        let buffer = storage::Buffer(&agents);
                        &buffer.into_bytes()
                    },
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                })
            },
            render_texture: {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Output texture"),
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba32Float,
                    usage: wgpu::TextureUsages::STORAGE_BINDING,
                    view_formats: &[wgpu::TextureFormat::Rgba32Float],
                });

                texture.create_view(&wgpu::TextureViewDescriptor::default())
            },
        };

        let gui_layer = {
            let ctx = egui::Context::default();
            let state = egui_winit::State::new(window);
            let renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1);

            GuiLayer {
                ctx,
                state,
                renderer,
                interface: gui::Interface::new(),
                enabled: true,
            }
        };

        Self {
            globals,
            surface,
            device,
            queue,
            config,
            size,
            pipelines,
            pipeline_data,
            gui_layer,
        }
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0
            && new_size.height > 0
            && new_size.width < u32::MAX
            && new_size.height < u32::MAX
        {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        let mut handled = self
            .gui_layer
            .state
            .on_event(&self.gui_layer.ctx, event)
            .consumed;

        match event {
            WindowEvent::KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        virtual_keycode: Some(winit::event::VirtualKeyCode::Space),
                        state,
                        ..
                    },
                ..
            } => {
                if *state == winit::event::ElementState::Released {
                    self.gui_layer.enabled = !self.gui_layer.enabled;
                    handled = true;
                }
            }
            _ => {}
        }

        handled
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        {
            let now = std::time::Instant::now();
            let prev_time = self.globals.timing.time;
            self.globals.timing.time = now;
            self.globals.timing.time_since_last_frame = now - prev_time;
            self.globals.timing.frame += 1;
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut cmd_buffer = Vec::new();

        // Copy globals to GPU
        {
            let bytes = {
                let storage: storage::Globals = (&self.globals).into();
                storage::Uniform(&storage).into_bytes()
            };

            let globals_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: &bytes,
                        usage: wgpu::BufferUsages::COPY_SRC,
                    });

            encoder.copy_buffer_to_buffer(
                &globals_buffer,
                0,
                &self.pipeline_data.globals_buffer,
                0,
                bytes.len() as wgpu::BufferAddress,
            );
        }

        // Diffuse pass
        {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Diffuse bind group"),
                layout: &self.pipelines.diffuse.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.pipeline_data.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &self.pipeline_data.render_texture,
                        ),
                    },
                ],
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Diffuse pass"),
                });

                compute_pass.set_pipeline(&self.pipelines.diffuse.pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.dispatch_workgroups(self.size.width, self.size.height, 1)
            }
        }

        // Simulation pass
        {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Simulation bind group"),
                layout: &self.pipelines.simulation.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.pipeline_data.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.pipeline_data.agents_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &self.pipeline_data.render_texture,
                        ),
                    },
                ],
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Simulation pass"),
                });

                compute_pass.set_pipeline(&self.pipelines.simulation.pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.dispatch_workgroups(
                    self.globals.work_groups.x,
                    self.globals.work_groups.y,
                    self.globals.work_groups.z,
                );
            }
        }

        // Render pass
        {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render bind group"),
                layout: &self.pipelines.render.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.pipeline_data.render_texture,
                    ),
                }],
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[
                        // This is what @location(0) in the fragment shader targets
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 1.0,
                                    g: 0.0,
                                    b: 0.5,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        }),
                    ],
                    depth_stencil_attachment: None,
                });

                render_pass.set_pipeline(&self.pipelines.render.pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.pipeline_data.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.pipeline_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );

                render_pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        // GUI Pass
        if self.gui_layer.enabled {
            let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
                size_in_pixels: [self.size.width as u32, self.size.height as u32],
                pixels_per_point: self.gui_layer.state.pixels_per_point(),
            };

            let input = self.gui_layer.state.take_egui_input(window);
            let output = self.gui_layer.ctx.run(input, |ctx| {
                self.gui_layer.interface.ui(ctx, &mut self.globals);
            });

            self.gui_layer.state.handle_platform_output(
                window,
                &self.gui_layer.ctx,
                output.platform_output,
            );

            let texture_deltas = output.textures_delta;
            let paint_jobs = self.gui_layer.ctx.tessellate(output.shapes);

            for (id, image_delta) in &texture_deltas.set {
                self.gui_layer
                    .renderer
                    .update_texture(&self.device, &self.queue, *id, image_delta);
            }

            for id in &texture_deltas.free {
                self.gui_layer.renderer.free_texture(id);
            }

            let gui_commands = self.gui_layer.renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &paint_jobs,
                &screen_descriptor,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GUI Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.gui_layer
                .renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);

            cmd_buffer.extend(gui_commands.into_iter());
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(
            cmd_buffer
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        output.present();

        Ok(())
    }
}

fn random_unit_circle() -> glam::f32::Vec2 {
    let mut rng = rand::thread_rng();
    let theta = rng.gen_range(0.0..std::f32::consts::TAU);
    glam::f32::vec2(theta.cos(), theta.sin())
}

impl Timing {
    pub fn dt(&self) -> f32 {
        self.time_since_last_frame.as_secs_f32()
    }
}

impl From<&Globals> for storage::Globals {
    fn from(globals: &Globals) -> Self {
        Self {
            dt: globals.timing.dt(),
            time: (globals.timing.time - globals.timing.start_time).as_secs_f32(),
            work_group_size: globals.work_groups.x,
        }
    }
}
