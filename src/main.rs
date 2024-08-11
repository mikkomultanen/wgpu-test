mod gui;
pub mod sdf;
mod renderer;
mod egui_renderer;

use glam::*;
use egui_renderer::EguiRenderer;
use egui_wgpu::ScreenDescriptor;
use wgpu::{Device, Queue, TextureFormat, TextureView};
use std::{sync::Arc, time::Instant};
use winit::{
    event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{Window, WindowBuilder}
};
use renderer::light::LightData;
use renderer::shape::ShapeData;

const WINDOW_SIZE: winit::dpi::LogicalSize<u32> = winit::dpi::LogicalSize::new(1280, 720);
const WORLD_SIZE: Vec2 = Vec2::new(256.0, 256.0);
const SDF_SIZE: UVec2 = UVec2::new(1024, 1024);

struct State {
    size: winit::dpi::PhysicalSize<u32>,
    renderer_scale: f32,
    scale_factor: f64,
    gui: gui::GUI,
    sdf: sdf::SDF,
    renderer: renderer::Renderer,
    egui_renderer: EguiRenderer,
    lights: Vec<LightData>,
    shapes: Vec<ShapeData>,
    mouse_pos: Vec2,
    add_pressed: bool,
    subtract_pressed: bool,
    up_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    down_pressed: bool,
    zoom_in_pressed: bool,
    zoom_out_pressed: bool,
    add_light_pressed: bool,
    add_shape_pressed: bool,
    add_entity_pressed: bool,
}

impl State {
    fn new(window: &Window, device: &Device, queue: &Queue, surface_format: TextureFormat) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let sdf = sdf::SDF::new(SDF_SIZE, WORLD_SIZE, &device, &queue);

        let mut lights = Vec::new();
        lights.push(LightData::new([1., 1., 1.], [0., 0.], 10., 10. / 40. * 0.5 * WORLD_SIZE.x));
        let mut shapes = Vec::new();
        shapes.push(ShapeData::new());

        let mut gui = gui::GUI::new(&window);
        gui.update_lights(lights.len());
        gui.update_shapes(shapes.len());

        let renderer_scale = gui.renderer_scale;
        let render_resolution = UVec2::new(
            ((size.width as f32 * renderer_scale).ceil() as u32).clamp(16, size.width),
            ((size.height as f32 * renderer_scale).ceil() as u32).clamp(16, size.height),
        );
        let output_resolution = UVec2::new(size.width, size.height);
        let renderer = renderer::Renderer::new(render_resolution, output_resolution, WORLD_SIZE, device, queue, &sdf, &surface_format);

        let egui_renderer = EguiRenderer::new(&device, surface_format, None, 1, &window);

        gui.update_res(render_resolution, output_resolution);

        Self {
            size,
            renderer_scale,
            scale_factor,
            gui,
            sdf,
            renderer,
            egui_renderer,
            lights,
            shapes,
            mouse_pos: Vec2::ZERO,
            add_pressed: false,
            subtract_pressed: false,
            up_pressed: false,
            left_pressed: false,
            right_pressed: false,
            down_pressed: false,
            zoom_in_pressed: false,
            zoom_out_pressed: false,
            add_light_pressed: false,
            add_shape_pressed: false,
            add_entity_pressed: false,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, new_scale_factor: f64, device: &Device) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.scale_factor = new_scale_factor;
            let renderer_scale = self.gui.renderer_scale;
            let render_resolution = UVec2::new(
                ((new_size.width as f32 * renderer_scale).ceil() as u32).clamp(16, 8192),
                ((new_size.height as f32 * renderer_scale).ceil() as u32).clamp(16, 8192),
            );
            let output_resolution = UVec2::new(new_size.width, new_size.height);
            self.renderer.resize(render_resolution, output_resolution, device);
            self.egui_renderer.ppp(self.scale_factor as f32);
            self.gui.update_res(render_resolution, output_resolution);
        }
    }

    fn input(&mut self, event: &WindowEvent, gui_captured: bool) -> bool {
        if gui_captured {
            self.add_pressed = false;
            self.subtract_pressed = false;
        }
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let size = self.size;
                let normalized_x = position.x as f32 / size.width as f32;
                let normalized_y = position.y as f32 / size.height as f32;
                self.mouse_pos = Vec2::new(
                    normalized_x - 0.5,
                    0.5 - normalized_y
                );
                true
            }
            WindowEvent::MouseInput { state, button, ..} => if !gui_captured {
                let pressed = *state == ElementState::Pressed;
                match *button {
                    MouseButton::Left => self.add_pressed = pressed,
                    MouseButton::Right => self.subtract_pressed = pressed,
                    _ => (),
                }
                true
            } else {
                false
            }
            WindowEvent::KeyboardInput { event, ..} => {
                let pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => { self.up_pressed = pressed; true},
                    PhysicalKey::Code(KeyCode::KeyA) => { self.left_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyD) => { self.right_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyS) => { self.down_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyZ) => { self.zoom_in_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyX) => { self.zoom_out_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyL) => { self.add_light_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyO) => { self.add_shape_pressed = pressed; true },
                    PhysicalKey::Code(KeyCode::KeyE) => { self.add_entity_pressed = pressed; true },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn update(&mut self, frame_time: f32) {
        let mut d: Vec2 = Vec2::ZERO;
        if self.up_pressed { d += Vec2::Y; }
        if self.left_pressed { d += -Vec2::X; }
        if self.right_pressed { d += Vec2::X; }
        if self.down_pressed { d += -Vec2::Y; }
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

        if self.add_shape_pressed {
            self.add_shape_pressed = false;
            if self.shapes.len() < renderer::MAX_SHAPES {
                self.shapes.push(self.shapes[0].clone());
                self.gui.update_shapes(self.shapes.len());
            }
        }

        if self.add_entity_pressed {
            self.add_entity_pressed = false;
            let count = renderer::MAX_SHAPES / 11;
            let s = (count as f32 / (WORLD_SIZE.x * WORLD_SIZE.y)).sqrt();
            let w = (s * WORLD_SIZE.x).ceil();
            let h = (s * WORLD_SIZE.y).ceil();
            let mut i = 0.;
            let mut j = 0.;
            while i < w {
                while j < h {
                    let x = ((i + 0.5) / w - 0.5) * WORLD_SIZE.x;
                    let y = ((j + 0.5) / h - 0.5) * WORLD_SIZE.y;
                    let position = Vec3::new(x, y, -2.);
                    self.add_entity(position);
                    j = j + 1.;
                }
                j = 0.;
                i = i + 1.;
            }
            //self.add_entity(self.mouse_world_pos().to_vec().extend(-2.));
        }
    }

    fn add_entity(&mut self, position: Vec3) {
        if self.shapes.len() + 11 <= renderer::MAX_SHAPES {
            let mut parts: Vec<ShapeData> = vec![ShapeData::new(); 11];
            parts[0].update_rounded_cone([0., 0., 0.35].into(), 0.35, [0., 0., 0.6].into(), 0.25, [1., 0.5, 0.], 0., 0.8,);
            parts[1].update_rounded_cone([0., 0., 0.6].into(), 0.15, [0.32, 0., 0.6].into(), 0.075, [0.8, 0.8, 0.8], 0., 0.8,);
            parts[2].update_sphere([0.36, 0., 0.66].into(), 0.03, [0., 0., 0.], 0., 0.2,);
            parts[3].update_sphere([0.19, 0.1, 0.7].into(), 0.03, [0., 0., 0.], 0., 0.2,);
            parts[4].update_sphere([0.19, -0.1, 0.7].into(), 0.03, [0., 0., 0.], 0., 0.2,);
            parts[5].update_rounded_cone([0., -0.15, 0.4].into(), 0.2, [0., -0.45, 0.45].into(), 0.08, [1., 0.5, 0.], 0., 0.8,);
            parts[6].update_rounded_cone([0., 0.15, 0.4].into(), 0.2, [0., 0.45, 0.45].into(), 0.08, [1., 0.5, 0.], 0., 0.8,);
            parts[7].update_sphere([0.07, 0., 0.33].into(), 0.3, [0.8, 0.8, 0.8], 0., 0.2,);
            parts[8].update_rounded_cone([0., 0., 0.35].into(), 0.2, [-0.3, 0., 0.1].into(), 0.05, [1., 0.5, 0.], 0., 0.8,);
            parts[9].update_rounded_cone([-0.05, -0.15, 0.05].into(), 0.1, [0.2, -0.2, 0.01].into(), 0.1, [1., 0.5, 0.], 0., 0.8,);
            parts[10].update_rounded_cone([-0.05, 0.15, 0.05].into(), 0.1, [0.2, 0.2, 0.01].into(), 0.1, [1., 0.5, 0.], 0., 0.8, );
            for part in parts.iter_mut() {
                part.translate(position);
            }
            self.shapes.append(&mut parts);
            self.gui.update_shapes(self.shapes.len());
        }
    } 

    fn mouse_world_pos(&self) -> Vec2 {
        wrap(self.mouse_pos.mul_add(self.renderer.view_size, self.renderer.position))
    }

    fn render(&mut self, view: &TextureView, device: &Device, queue: &Queue, window: &Window) {
        let renderer_scale = self.gui.renderer_scale;
        if renderer_scale != self.renderer_scale {
            self.renderer_scale = renderer_scale;
            let render_resolution = UVec2::new(
                ((self.size.width as f32 * renderer_scale).ceil() as u32).clamp(16, self.size.width),
                ((self.size.height as f32 * renderer_scale).ceil() as u32).clamp(16, self.size.height),
            );
            let output_resolution = UVec2::new(self.size.width, self.size.height);
            self.renderer.resize_render_resolution(render_resolution, &device);
            self.gui.update_res(render_resolution, output_resolution);
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let mouse_world_pos = self.mouse_world_pos();
        let cursor_size = self.gui.cursor_size;
        self.lights[0].update(
            self.gui.light_color(), 
            mouse_world_pos.into(),
            self.gui.light_radius,
            (self.gui.light_range * 0.5 * WORLD_SIZE.x.min(WORLD_SIZE.y)).max(self.gui.light_radius),
        );
        self.shapes[0].update_sphere(
            mouse_world_pos.extend(-2. + self.gui.shape_radius),
            self.gui.shape_radius,
            self.gui.shape_color,
            self.gui.shape_metallic,
            self.gui.shape_roughness,
        );

        if self.add_pressed {
            self.sdf.add(
                mouse_world_pos, 
                cursor_size, 
                queue,
                &mut encoder,
            );
        }
        if self.subtract_pressed {
            self.sdf.subtract(
                mouse_world_pos, 
                cursor_size, 
                queue,
                &mut encoder,
            )
        }

        self.renderer.update_uniforms(
            mouse_world_pos,
            cursor_size,
            self.gui.exposure,
        );
        self.renderer.update_lights(queue, &self.lights);
        self.renderer.update_shapes(queue, &mut self.shapes);
        self.renderer.update_upsampler(device, queue, &self.gui.upsampler);
        self.renderer.render(device, queue, &mut encoder, &self.sdf, &self.shapes, &view);
        
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.scale_factor as f32,
        };

        let egui_renderer = &mut self.egui_renderer;
        let gui = &mut self.gui;
        egui_renderer.draw(device, queue, &mut encoder, window, view, screen_descriptor, |ctx| gui.draw(ctx));

        queue.submit(std::iter::once(encoder.finish()));
    }
}

fn wrap(p: Vec2) -> Vec2 {
    let sx = (p.x / WORLD_SIZE.x).abs().ceil() + 0.5;
    let x = (p.x + sx * WORLD_SIZE.x) % WORLD_SIZE.x - 0.5 * WORLD_SIZE.x;
    let sy = (p.y / WORLD_SIZE.y).abs().ceil() + 0.5;
    let y = (p.y + sy * WORLD_SIZE.y) % WORLD_SIZE.y - 0.5 * WORLD_SIZE.y;
    Vec2::new(x, y)
}

fn main() {
    pollster::block_on(run());
}

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("New event loop");
    let window = WindowBuilder::new()
    .with_title("WGPU test")
    .with_inner_size(WINDOW_SIZE)
    .build(&event_loop).expect("New window");
    let window = Arc::new(window);

    let size = window.inner_size();

    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window.clone()).expect("New surface");
    let adapter = instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        },
    ).await.expect("No suitable GPU adapters found on the system!");

    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..Default::default()
            },
            label: None,
        },
        None,
    ).await.expect("Unable to find a suitable GPU adapter!");

    let mut config = surface.get_default_config(&adapter, size.width, size.height).expect("Default surface config");

    surface.configure(&device, &config);

    let mut state = State::new(&window, &device, &queue, config.format);
    let mut last_frame_inst = Instant::now();
    let (mut frame_count, mut accum_time) = (0, 0.0);

    event_loop.set_control_flow(ControlFlow::Wait);

    let _ = event_loop.run(move |winit_event, elwt| {
        match winit_event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            },
            Event::AboutToWait => {
                window.request_redraw();
            },
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let gui_captured = state.egui_renderer.handle_input(&window, &event);
                if !state.input(event, gui_captured) {
                    match event {
                        WindowEvent::Resized(_) => {
                            let new_size = window.inner_size();
                            config.width = new_size.width;
                            config.height = new_size.height;
                            surface.configure(&device, &config);
                            state.resize(window.inner_size(), window.scale_factor(), &device);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            let new_size = window.inner_size();
                            config.width = new_size.width;
                            config.height = new_size.height;
                            surface.configure(&device, &config);
                            state.resize(window.inner_size(), window.scale_factor(), &device);
                        }
                        WindowEvent::Focused(new_focused) => {
                            if *new_focused {
                                window.request_redraw();
                            }
                        }
                        WindowEvent::RedrawRequested => {
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

                            let present_mode = state.gui.present_mode();
                            if present_mode != config.present_mode {
                                config.present_mode = present_mode;
                                surface.configure(&device, &config);
                            }

                            let frame = surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                
                            state.render(&view, &device, &queue, &window);

                            frame.present();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}