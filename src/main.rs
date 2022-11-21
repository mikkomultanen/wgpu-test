mod gui;
mod sdf;
mod renderer;

use cgmath::*;
use std::time::Instant;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder}
};
use renderer::light::LightData;

const WINDOW_SIZE: winit::dpi::LogicalSize<u32> = winit::dpi::LogicalSize::new(1280, 720);
const WORLD_SIZE: Vector2<f32> = Vector2::new(1024.0, 1024.0);
const SDF_SIZE: u32 = 1024;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    renderer_scale: f32,
    scale_factor: f64,
    gui: gui::GUI,
    sdf: sdf::SDF,
    renderer: renderer::Renderer,
    lights: Vec<LightData>,
    mouse_pos: Point2<f32>,
    mouse_pressed: bool,
    up_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    down_pressed: bool,
    zoom_in_pressed: bool,
    zoom_out_pressed: bool,
    add_light_pressed: bool,
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
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ).await.expect("No suitable GPU adapters found on the system!");

        let (device, mut queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.expect("Unable to find a suitable GPU adapter!");

        let surface_format = surface.get_supported_formats(&adapter)[0];

        let sdf = sdf::SDF::new(SDF_SIZE, WORLD_SIZE, &device, &queue);

        let mut lights = Vec::new();
        lights.push(LightData::new([1., 1., 1.], [0., 0.], 10., 10. / 40. * 0.5 * WORLD_SIZE.x));

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        };

        surface.configure(&device, &config);

        let mut gui = gui::GUI::new(window, &device, &surface_format);
        gui.update_lights(lights.len());

        let renderer_scale = gui.renderer_scale;
        let render_resolution = Vector2::new(
            ((size.width as f32 * renderer_scale).ceil() as u32).clamp(16, size.width),
            ((size.height as f32 * renderer_scale).ceil() as u32).clamp(16, size.height),
        );
        let output_resolution = Vector2::new(size.width, size.height);
        let renderer = renderer::Renderer::new(render_resolution, output_resolution, WORLD_SIZE, &device, &mut queue, &sdf.view, &sdf.sampler, &surface_format);

        gui.update_res(render_resolution, output_resolution);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            renderer_scale,
            scale_factor,
            gui,
            sdf,
            renderer,
            lights,
            mouse_pos: Point2::origin(),
            mouse_pressed: false,
            up_pressed: false,
            left_pressed: false,
            right_pressed: false,
            down_pressed: false,
            zoom_in_pressed: false,
            zoom_out_pressed: false,
            add_light_pressed: false,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, new_scale_factor: f64) {
        if new_size.width > 0 && new_size.height > 0 {
            self.gui.resize(&new_size, new_scale_factor);
            self.size = new_size;
            self.scale_factor = new_scale_factor;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            let renderer_scale = self.gui.renderer_scale;
            let render_resolution = Vector2::new(
                ((new_size.width as f32 * renderer_scale).ceil() as u32).clamp(16, new_size.width),
                ((new_size.height as f32 * renderer_scale).ceil() as u32).clamp(16, new_size.height),
            );
            let output_resolution = Vector2::new(new_size.width, new_size.height);
            self.renderer.resize(render_resolution, output_resolution, &self.device);
            self.gui.update_res(render_resolution, output_resolution);
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
                self.mouse_pos = Point2::new(
                    normalized_x - 0.5,
                    0.5 - normalized_y
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
                    Some(VirtualKeyCode::W) => { self.up_pressed = pressed; true},
                    Some(VirtualKeyCode::A) => { self.left_pressed = pressed; true },
                    Some(VirtualKeyCode::D) => { self.right_pressed = pressed; true },
                    Some(VirtualKeyCode::S) => { self.down_pressed = pressed; true },
                    Some(VirtualKeyCode::Z) => { self.zoom_in_pressed = pressed; true },
                    Some(VirtualKeyCode::X) => { self.zoom_out_pressed = pressed; true },
                    Some(VirtualKeyCode::L) => { self.add_light_pressed = pressed; true },
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
        let mut z = 1.0;
        if self.zoom_in_pressed { z *= 0.5f32.powf(frame_time); }
        if self.zoom_out_pressed { z /= 0.5f32.powf(frame_time); }
        self.renderer.position = wrap(self.renderer.position + d);
        self.renderer.view_size *= z;

        if self.add_light_pressed {
            self.add_light_pressed = false;
            if self.lights.len() < renderer::MAX_LIGHTS {
                self.lights.push(self.lights[0].clone());
                self.gui.update_lights(self.lights.len());
            }
        }
    }

    fn render(&mut self) -> Result<(), String> {
        let present_mode = self.gui.present_mode();
        if present_mode != self.config.present_mode {
            self.config.present_mode = present_mode;
            self.surface.configure(&self.device, &self.config);
        }
        let renderer_scale = self.gui.renderer_scale;
        if renderer_scale != self.renderer_scale {
            self.renderer_scale = renderer_scale;
            let render_resolution = Vector2::new(
                ((self.size.width as f32 * renderer_scale).ceil() as u32).clamp(16, self.size.width),
                ((self.size.height as f32 * renderer_scale).ceil() as u32).clamp(16, self.size.height),
            );
            let output_resolution = Vector2::new(self.size.width, self.size.height);
            self.renderer.resize_render_resolution(render_resolution, &self.device);
            self.gui.update_res(render_resolution, output_resolution);
        }
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let mouse_world_pos = wrap(self.renderer.position + self.mouse_pos.to_vec().mul_element_wise(self.renderer.view_size));
        let cursor_size = self.gui.cursor_size;
        self.lights[0].update(
            self.gui.light_color(), 
            [mouse_world_pos.x, mouse_world_pos.y],
            self.gui.light_radius,
            (self.gui.light_range * 0.5 * WORLD_SIZE.x).max(self.gui.light_radius),
        );

        if self.mouse_pressed {
            self.sdf.add(
                mouse_world_pos, 
                cursor_size, 
                &self.queue,
                &mut encoder,
            );
        }

        self.renderer.update_uniforms(
            mouse_world_pos,
            cursor_size,
            self.gui.exposure,
        );
        self.renderer.update_lights(&mut self.queue, &self.lights);
        self.renderer.update_upsampler(&self.device, &mut self.queue, &self.gui.upsampler);
        self.renderer.render(&mut self.device, &mut self.queue, &mut encoder, &view);
        
        self.gui.render(&mut self.device, &mut self.queue, &mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));
    
        frame.present();
        
        Ok(())
    }
}

fn wrap(p: Point2<f32>) -> Point2<f32> {
    let sx = (p.x / WORLD_SIZE.x).abs().ceil() + 0.5;
    let x = (p.x + sx * WORLD_SIZE.x) % WORLD_SIZE.x - 0.5 * WORLD_SIZE.x;
    let sy = (p.y / WORLD_SIZE.y).abs().ceil() + 0.5;
    let y = (p.y + sy * WORLD_SIZE.y) % WORLD_SIZE.y - 0.5 * WORLD_SIZE.y;
    Point2::new(x, y)
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
    .with_title("WGPU test")
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