use glam::UVec2;
use egui::Context;

use std::string::String;

use crate::renderer;

pub struct GUI {
    pub cursor_size: f32,
    light_hue: f32,
    light_saturation: f32,
    light_intensity: f32,
    pub light_radius: f32,
    pub light_range: f32,
    pub exposure: f32,
    pub shape_color: [f32; 3],
    pub shape_metallic: f32,
    pub shape_roughness: f32,
    pub shape_radius: f32,
    pub upsampler: renderer::Upsampler,
    pub renderer_scale: f32,
    v_sync: bool,
    fps_str: String,
    res_str: String,
    lights_str: String,
    shapes_str: String,
}

fn res_str(render_resolution: UVec2, output_resolution: UVec2) -> String {
    return format!("R: {}x{} O: {}x{}", render_resolution.x, render_resolution.y, output_resolution.x, output_resolution.y);
}

impl GUI {
    pub fn new(
        window: &winit::window::Window,
    ) -> Self {
        return Self {
            cursor_size: 1.0,
            light_hue: 0.0,
            light_saturation: 0.0,
            light_intensity: 100.0,
            light_radius: 0.1,
            light_range: 1.0,
            exposure: 1.0,
            shape_color: [0.5, 1.0, 0.5],
            shape_metallic: 0.,
            shape_roughness: 0.1,
            shape_radius: 0.5,
            upsampler: renderer::Upsampler::BLIT,
            renderer_scale: 1.0 / (window.scale_factor() as f32), 
            v_sync: true,
            fps_str: format!("FPS: -"),
            res_str: format!("R. - O: -"),
            lights_str: format!("LIGHTS: -"),
            shapes_str: format!("SHAPES: -"),
        }
    }

    pub fn update_fps(&mut self, fps: f32) {
        self.fps_str = format!("FPS: {:.1}", fps);
    }

    pub fn update_lights(&mut self, num_lights: usize) {
        self.lights_str = format!("LIGHTS: {}", num_lights);
    }

    pub fn update_shapes(&mut self, num_shapes: usize) {
        self.shapes_str = format!("SHAPES: {}", num_shapes);
    }

    pub fn update_res(&mut self, render_resolution: UVec2, output_resolution: UVec2) {
        self.res_str = res_str(render_resolution, output_resolution);
    }

    pub fn draw(&mut self, ctx: &Context) {
        egui::Window::new("Stats")
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Checkbox::new(&mut self.v_sync, "VSync"));
                ui.label(self.fps_str.as_str());
                ui.add(egui::Slider::new(&mut self.renderer_scale, 0.5..=1.0).step_by(1.0/32.0).show_value(false));
                ui.label(self.res_str.as_str());
                ui.label(self.lights_str.as_str());
                ui.label(self.shapes_str.as_str());
            });
        });

        egui::Window::new("Tools")
        .resizable(false)
        .show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.cursor_size, 1.0..=10.0).text("cursor size"));
            ui.add(egui::Slider::new(&mut self.light_hue, 0.0..=1.0).text("light hue"));
            ui.add(egui::Slider::new(&mut self.light_saturation, 0.0..=1.0).text("light saturation"));
            ui.add(egui::Slider::new(&mut self.light_intensity, 0.0..=1000.0).text("light intensity"));
            ui.add(egui::Slider::new(&mut self.light_radius, 0.0..=1.0).text("light radius"));
            ui.add(egui::Slider::new(&mut self.light_range, 0.0..=1.0).text("light range"));
            ui.add(egui::Slider::new(&mut self.exposure, 0.0..=100.0).text("exposure"));
            egui::widgets::color_picker::color_edit_button_rgb(ui, &mut self.shape_color);
            ui.add(egui::Slider::new(&mut self.shape_metallic, 0.0..=1.0).text("shape metallic"));
            ui.add(egui::Slider::new(&mut self.shape_roughness, 0.0..=1.0).text("shape roughness"));
            ui.add(egui::Slider::new(&mut self.shape_radius, 0.0..=1.0).text("shape radius"));
            egui::ComboBox::from_label("upsampler")
            .selected_text(format!("{:?}", self.upsampler))
            .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.upsampler, renderer::Upsampler::TAA, format!("{:?}", renderer::Upsampler::TAA));
                        ui.selectable_value(&mut self.upsampler, renderer::Upsampler::BLIT, format!("{:?}", renderer::Upsampler::BLIT));
                    });
        });
    }

    pub fn light_color(&self) -> [f32; 3] {
        return egui::ecolor::rgb_from_hsv((self.light_hue, self.light_saturation, self.light_intensity));
    }

    pub fn present_mode(&self) -> wgpu::PresentMode {
        if self.v_sync {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        }
    }
}
