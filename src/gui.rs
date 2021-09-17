use std::time::Instant;

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
    cursor_size: f32,
    v_sync: bool,
    fps_str: String,
    res_str: String,
}

fn res_str(size: &winit::dpi::PhysicalSize<u32>, scale_factor: f64) -> String {
    let log_size: winit::dpi::LogicalSize<u32> = size.to_logical(scale_factor);
    return format!("RES: {}x{} LM: {}x{}", size.width, size.height, log_size.width, log_size.height);
}

impl GUI {
    pub fn new(window: &Window, device: &wgpu::Device, surface_format: &wgpu::TextureFormat) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });    

        let rpass = RenderPass::new(&device, *surface_format, 1);

        let start_time = Instant::now();

        let cursor_size = 20.0;

        return Self {
            size,
            scale_factor,
            platform,
            rpass,
            start_time,
            cursor_size,
            v_sync: true,
            fps_str: format!("FPS: -"),
            res_str: res_str(&size, scale_factor),
        }
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, scale_factor: f64) {
        self.size = *size;
        self.scale_factor = scale_factor;
        self.res_str = res_str(size, scale_factor);
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

    pub fn render(&mut self, device: &mut wgpu::Device, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) -> egui::Output {
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
                    ui.label(self.res_str.as_str());
                });
            });

            egui::Window::new("Tools")
            .resizable(false)
            .show(&ctx, |ui| {
                ui.add(egui::Slider::new(&mut self.cursor_size, 5.0..=40.0).text("cursor size"));
            });
        }

        let (output, paint_commands) = self.platform.end_frame();
        let paint_jobs = self.platform.context().tessellate(paint_commands);

        let screen_descriptor = ScreenDescriptor {
            physical_width: self.size.width,
            physical_height: self.size.height,
            scale_factor: self.scale_factor as f32,
        };
        self.rpass.update_texture(&device, &queue, &self.platform.context().texture());
        self.rpass.update_user_textures(&device, &queue);
        self.rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        self.rpass.execute(
            encoder,
            &view,
            &paint_jobs,
            &screen_descriptor,
            None,
        ).unwrap();

        output
    }

    pub fn cursor_size(&self) -> f32 {
        self.cursor_size * self.scale_factor as f32
    }

    pub fn present_mode(&self) -> wgpu::PresentMode {
        if self.v_sync {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Immediate
        }
    }
}
