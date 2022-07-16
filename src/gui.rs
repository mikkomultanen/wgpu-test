use std::time::Instant;

use cgmath::Vector2;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use std::string::String;
use winit::window::Window;

pub struct GUI {
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    platform: Platform,
    rpass: RenderPass,
    start_time: Instant,
    pub cursor_size: f32,
    light_hue: f32,
    light_saturation: f32,
    light_intensity: f32,
    pub light_radius: f32,
    pub light_range: f32,
    pub exposure: f32,
    pub renderer_scale: f32,
    v_sync: bool,
    fps_str: String,
    res_str: String,
    lights_str: String,
}

fn res_str(render_resolution: Vector2<u32>, output_resolution: Vector2<u32>) -> String {
    return format!("R: {}x{} O: {}x{}", render_resolution.x, render_resolution.y, output_resolution.x, output_resolution.y);
}

impl GUI {
    pub fn new(window: &Window, device: &wgpu::Device, surface_format: &wgpu::TextureFormat) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });    

        let rpass = RenderPass::new(&device, *surface_format, 1);

        return Self {
            size,
            scale_factor,
            platform,
            rpass,
            start_time: Instant::now(),
            cursor_size: 20.0,
            light_hue: 0.0,
            light_saturation: 0.0,
            light_intensity: 1.0,
            light_radius: 10.0,
            light_range: 1.0,
            exposure: 1.0,
            renderer_scale: 0.75, 
            v_sync: true,
            fps_str: format!("FPS: -"),
            res_str: format!("R. - O: -"),
            lights_str: format!("LIGHTS: -"),
        }
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, scale_factor: f64) {
        self.size = *size;
        self.scale_factor = scale_factor;
    }

    pub fn input<T>(&mut self, event: &winit::event::Event<T>) -> bool {
        self.platform.handle_event(event);
        return self.platform.captures_event(event);
    }

    pub fn update(&mut self) {
        self.platform.update_time(self.start_time.elapsed().as_secs_f64());
    }

    pub fn update_fps(&mut self, fps: f32) {
        self.fps_str = format!("FPS: {:.1}", fps);
    }

    pub fn update_lights(&mut self, num_lights: usize) {
        self.lights_str = format!("LIGHTS: {}", num_lights);
    }

    pub fn update_res(&mut self, render_resolution: Vector2<u32>, output_resolution: Vector2<u32>) {
        self.res_str = res_str(render_resolution, output_resolution);
    }

    pub fn render(&mut self, device: &mut wgpu::Device, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.platform.begin_frame();

        {
            let ctx = self.platform.context();
            egui::Window::new("Stats")
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Checkbox::new(&mut self.v_sync, "VSync"));
                    ui.label(self.fps_str.as_str());
                    ui.add(egui::Slider::new(&mut self.renderer_scale, 0.5..=1.0).step_by(1.0/32.0).show_value(false));
                    ui.label(self.res_str.as_str());
                    ui.label(self.lights_str.as_str());
                });
            });

            egui::Window::new("Tools")
            .resizable(false)
            .show(&ctx, |ui| {
                ui.add(egui::Slider::new(&mut self.cursor_size, 5.0..=40.0).text("cursor size"));
                ui.add(egui::Slider::new(&mut self.light_hue, 0.0..=1.0).text("light hue"));
                ui.add(egui::Slider::new(&mut self.light_saturation, 0.0..=1.0).text("light saturation"));
                ui.add(egui::Slider::new(&mut self.light_intensity, 0.0..=10.0).text("light intensity"));
                ui.add(egui::Slider::new(&mut self.light_radius, 0.0..=40.0).text("light radius"));
                ui.add(egui::Slider::new(&mut self.light_range, 0.0..=1.0).text("light range"));
                ui.add(egui::Slider::new(&mut self.exposure, 0.0..=10.0).text("exposure"));
            });
        }

        let output = self.platform.end_frame(None);
        let paint_jobs = self.platform.context().tessellate(output.shapes);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.size.width,
            physical_height: self.size.height,
            scale_factor: self.scale_factor as f32,
        };
        self.rpass.add_textures(&device, &queue, &output.textures_delta).unwrap();
        self.rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        self.rpass.execute(
            encoder,
            &view,
            &paint_jobs,
            &screen_descriptor,
            None,
        ).unwrap();
        self.rpass.remove_textures(output.textures_delta).unwrap();
    }

    pub fn light_color(&self) -> [f32; 3] {
        return egui::color::rgb_from_hsv((self.light_hue, self.light_saturation, self.light_intensity));
    }

    pub fn present_mode(&self) -> wgpu::PresentMode {
        if self.v_sync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        }
    }
}
