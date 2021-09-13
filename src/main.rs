mod gui;
mod sdf;
mod renderer;

use cgmath::*;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder}
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniforms {
    pub translate: [f32; 2],
    pub view_size: [f32; 2],
    pub world_size: [f32; 2],
    pub mouse: [f32; 2],
    pub cursor_size: f32,
}

const WINDOW_SIZE: winit::dpi::LogicalSize<u32> = winit::dpi::LogicalSize::new(640, 640);
const RENDERER_SIZE: Vector2<u32> = Vector2::new(640, 640);
const WORLD_SIZE: Vector2<f32> = Vector2::new(1000.0, 1000.0);
const SDF_SIZE: u32 = 1024;

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms {
            translate: [0.0, 0.0],
            view_size: [WORLD_SIZE.x, WORLD_SIZE.y],
            world_size: [WORLD_SIZE.x, WORLD_SIZE.y],
            mouse: [0.0, 0.0],
            cursor_size: 20.0,
        }
    }
}


struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    render_pipeline: wgpu::RenderPipeline,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    gui: gui::GUI,
    sdf: sdf::SDF,
    renderer: renderer::Renderer,
    renderer_bind_group: wgpu::BindGroup,
    mouse_pos: Point2<f32>,
    mouse_pressed: bool,
    up_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    down_pressed: bool,
    start_time: Instant,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
        ).await.expect("No suitable GPU adapters found on the system!");

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.expect("Unable to find a suitable GPU adapter!");

        let surface_format = surface.get_preferred_format(&adapter).unwrap();

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniforms = Uniforms::default();
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                }
            ]
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("uniform_bind_group"),
        });

        let sdf = sdf::SDF::new(SDF_SIZE, WORLD_SIZE, &device, &queue);

        let renderer = renderer::Renderer::new(RENDERER_SIZE, WORLD_SIZE, &device, &sdf.view, &sdf.sampler);

        let renderer_texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("renderer_texture_bind_group_layout"),
            }
        );

        let renderer_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &renderer_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&renderer.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                    }
                ],
                label: Some("renderer_bind_group"),
            }
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &renderer_texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[surface_format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(&device, &config);

        let gui = gui::GUI::new(window, &device, &surface_format);

        let start_time = Instant::now();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            scale_factor,
            render_pipeline,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            gui,
            sdf,
            renderer,
            renderer_bind_group,
            mouse_pos: Point2::origin(),
            mouse_pressed: false,
            up_pressed: false,
            left_pressed: false,
            right_pressed: false,
            down_pressed: false,
            start_time,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, new_scale_factor: f64) {
        if new_size.width > 0 && new_size.height > 0 {
            self.gui.resize(&new_size, new_scale_factor);
            self.size = new_size;
            self.scale_factor = new_scale_factor;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent, gui_captured: bool) -> bool {
        if gui_captured {
            self.mouse_pressed = false;
        }
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let size = self.size;
                let normalized_x = position.x as f32 / size.width as f32;
                let normalized_y = position.y as f32 / size.height as f32;
                self.mouse_pos = self.renderer.position + Vector2::new(
                    (normalized_x - 0.5) * self.renderer.view_size.x,
                    (0.5 - normalized_y) * self.renderer.view_size.y
                );
                true
            }
            WindowEvent::MouseInput { state, button, ..} => if !gui_captured {
                let pressed = *state == ElementState::Pressed;
                match *button {
                    MouseButton::Left => self.mouse_pressed = pressed,
                    _ => (),
                }
                true
            } else {
                false
            }
            WindowEvent::KeyboardInput { input, ..} => {
                let pressed = input.state == ElementState::Pressed;
                match input.virtual_keycode {
                    Some(VirtualKeyCode::Up) => { self.up_pressed = pressed; true},
                    Some(VirtualKeyCode::Left) => { self.left_pressed = pressed; true },
                    Some(VirtualKeyCode::Right) => { self.right_pressed = pressed; true },
                    Some(VirtualKeyCode::Down) => { self.down_pressed = pressed; true },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn update(&mut self, frame_time: f32) {
        self.gui.update();
        let mut d: Vector2<f32> = Vector2::zero();
        if self.up_pressed { d += Vector2::unit_y(); }
        if self.left_pressed { d += -Vector2::unit_x(); }
        if self.right_pressed { d += Vector2::unit_x(); }
        if self.down_pressed { d += -Vector2::unit_y(); }
        d *= 0.2 * self.renderer.view_size.x * frame_time;
        self.renderer.position = wrap(self.renderer.position + d);
        self.mouse_pos = wrap(self.mouse_pos + d);
        let present_mode = self.gui.present_mode();
        if present_mode != self.config.present_mode {
            self.config.present_mode = present_mode;
            self.surface.configure(&self.device, &self.config);
        }
        self.uniforms.translate = [self.renderer.position.x, self.renderer.position.y];
        self.uniforms.view_size = [self.renderer.view_size.x, self.renderer.view_size.y];
        self.uniforms.cursor_size = self.gui.cursor_size();
        self.uniforms.mouse = [self.mouse_pos.x, self.mouse_pos.y];
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
        if self.mouse_pressed {
            self.sdf.add(
                self.uniforms.mouse, 
                self.uniforms.cursor_size, 
                &self.device, 
                &self.queue
            );
        }
        self.renderer.update(
            self.uniforms.mouse, 
            self.start_time.elapsed().as_secs_f32(), 
            &self.device, 
            &self.queue
        );
    }

    fn render(&mut self) -> Result<(), String> {
        let frame = self
            .surface
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        }
                    }
                ],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        
        self.gui.render(&mut self.device, &mut self.queue, &mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));
    
        Ok(())
    }
}

fn wrap(p: Point2<f32>) -> Point2<f32> {
    let x = (p.x + 1.5 * WORLD_SIZE.x) % WORLD_SIZE.x - 0.5 * WORLD_SIZE.x;
    let y = (p.y + 1.5 * WORLD_SIZE.y) % WORLD_SIZE.y - 0.5 * WORLD_SIZE.y;
    Point2::new(x, y)
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
    .with_title("WGPU test")
    .with_resizable(false)
    .with_inner_size(WINDOW_SIZE)
    .build(&event_loop).unwrap();

    let mut state = pollster::block_on(State::new(&window));
    let mut last_frame_inst = Instant::now();
    let (mut frame_count, mut accum_time) = (0, 0.0);
    let mut focused = false;

    event_loop.run(move |winit_event, _, control_flow| {
        let gui_captured = state.gui.input(&winit_event);

        match winit_event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => if !state.input(event, gui_captured) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size, state.scale_factor);
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
                        state.resize(**new_inner_size, *scale_factor);
                    }
                    WindowEvent::Focused(new_focused) => {
                        focused = *new_focused;
                        if focused {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
            Event::RedrawEventsCleared => {
                if focused {
                    window.request_redraw();
                } else {
                    *control_flow = ControlFlow::Wait;
                }
            }
            Event::RedrawRequested(_) => {
                let frame_time = last_frame_inst.elapsed().as_secs_f32();
                accum_time += frame_time;
                last_frame_inst = Instant::now();
                frame_count += 1;
                if frame_count == 60 {
                    state.gui.update_fps(frame_count as f32 / accum_time);
                    accum_time = 0.0;
                    frame_count = 0;
                }

                state.update(frame_time);
                match state.render() {
                    Ok(_) => {}
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        }
    });
}