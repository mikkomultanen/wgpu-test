mod light_map;
mod texture;
mod taa;
pub mod light;

use cgmath::*;
use std::time::Instant;
use wgpu::util::DeviceExt;

use light_map::LightMapRenderer;

use crate::renderer::light::{LightData, LightsConfig};

pub const MAX_LIGHTS: usize = 1024;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniforms {
    pub translate: [f32; 2],
    pub view_size: [f32; 2],
    pub world_size: [f32; 2],
    pub inv_world_size: [f32; 2],
    pub pixel_size: [f32; 2],
    pub mouse: [f32; 2],
    pub cursor_size: f32,
    pub time: f32,
    pub exposure: f32,
}

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms {
            translate: [0.0, 0.0],
            view_size: [1.0, 1.0],
            world_size: [1.0, 1.0], 
            inv_world_size: [1.0, 1.0],
            pixel_size: [1.0, 1.0],
            mouse: [0.0, 0.0],
            cursor_size: 0.0,
            time: 0.0,
            exposure: 1.0,
        }
    }
}

pub struct Renderer {
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    lights_buffer: wgpu::Buffer,
    lights_config_buffer: wgpu::Buffer,
    lights_bind_group: wgpu::BindGroup,
    sdf_bind_group: wgpu::BindGroup,
    light_map_renderer: LightMapRenderer,
    lightmap_sampler: wgpu::Sampler,    
    lightmap_bind_group_layout: wgpu::BindGroupLayout,
    lightmap_bind_group: wgpu::BindGroup,
    color_texture: texture::Texture,
    blit_pipeline: wgpu::RenderPipeline,
    color_bind_group_layout: wgpu::BindGroupLayout,
    color_bind_group: wgpu::BindGroup,
    taa: taa::TAA,
    render_pipeline: wgpu::RenderPipeline,
    start_time: Instant,
    pub position: Point2<f32>,
    pub view_size: Vector2<f32>,
}

const COLOR_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

impl Renderer {
    pub fn new(render_resolution: Vector2<u32>, output_resolution: Vector2<u32>, world_size: Vector2<f32>, device: &wgpu::Device, queue: &mut wgpu::Queue, sdf_view: &wgpu::TextureView, sdf_sampler: &wgpu::Sampler, surface_format: &wgpu::TextureFormat) -> Self {
        let mut view_size = Vector2::new(world_size.x, world_size.y);
        view_size.x = view_size.y * output_resolution.x as f32 / output_resolution.y as f32;

        let mut uniforms = Uniforms::default();
        uniforms.view_size = [view_size.x, view_size.y];
        uniforms.world_size = [world_size.x, world_size.y];
        uniforms.inv_world_size = [1.0 / world_size.x, 1.0 / world_size.y];
        uniforms.pixel_size = [world_size.x / render_resolution.x as f32, world_size.y / render_resolution.y as f32];

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

        let sdf_bind_group_layout = device.create_bind_group_layout(
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
                label: Some("sdf_texture_bind_group_layout"),
            }
        );

        let sdf_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &sdf_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(sdf_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sdf_sampler),
                    }
                ],
                label: Some("sdf_bind_group"),
            }
        );

        let initial_lights_data = vec![LightData::default(); MAX_LIGHTS];
        let lights_config = LightsConfig { num_lights: 0, };

        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&initial_lights_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        });

        let lights_config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[lights_config]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let lights_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ]
        });

        let lights_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &lights_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: lights_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lights_config_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        let light_map_renderer = LightMapRenderer::new(render_resolution, device, queue, &uniform_bind_group_layout, &sdf_bind_group_layout, &lights_bind_group_layout);

        let lightmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let lightmap_bind_group_layout = device.create_bind_group_layout(
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
                label: Some("lightmap_bind_group_layout"),
            }
        );

        let lightmap_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &lightmap_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&light_map_renderer.lightmap_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&lightmap_sampler),
                    }
                ],
                label: Some("lightmap_bind_group"),
            }
        );

        let color_texture = texture::Texture::new_intermediate(device, render_resolution, COLOR_TEXTURE_FORMAT);

        let blit_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });

        let blit_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&lightmap_bind_group_layout],
                push_constant_ranges: &[],
            });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: COLOR_TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
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

        let color_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
                label: Some("renderer_texture_bind_group_layout"),
            }
        );

        let color_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &color_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&color_texture.view),
                    },
                ],
                label: Some("renderer_view_bind_group"),
            }
        );

        let taa = taa::TAA::new(output_resolution, device, queue, &uniform_bind_group_layout, &color_bind_group_layout);

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("renderer.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &sdf_bind_group_layout, &taa.output_bind_group_layout],
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
                targets: &[(*surface_format).into()],
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

        let start_time = Instant::now();
        let position = Point2::new(0., 0.);

        return Self {
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            lights_buffer,
            lights_config_buffer,
            lights_bind_group,
            sdf_bind_group,
            light_map_renderer,
            lightmap_sampler,
            lightmap_bind_group_layout,
            lightmap_bind_group,
            color_texture,
            blit_pipeline,
            color_bind_group_layout,
            color_bind_group,
            taa,
            render_pipeline,
            start_time,
            position,
            view_size,
        }
    }

    pub fn resize(&mut self, render_resolution: Vector2<u32>, output_resolution: Vector2<u32>, device: &wgpu::Device) {
        self.light_map_renderer.resize(render_resolution, device);
        self.lightmap_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &self.lightmap_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.light_map_renderer.lightmap_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.lightmap_sampler),
                    }
                ],
                label: Some("renderer_view_bind_group"),
            }
        );

        self.color_texture = texture::Texture::new_intermediate(device, render_resolution, COLOR_TEXTURE_FORMAT);
        self.color_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &self.color_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.color_texture.view),
                    },
                ],
                label: None,
            }
        );

        self.taa.resize(output_resolution);

        self.view_size.x = self.view_size.y * output_resolution.x as f32 / output_resolution.y as f32;
        self.uniforms.pixel_size = [self.view_size.x / output_resolution.x as f32, self.view_size.y / output_resolution.y as f32];
    }

    pub fn update_uniforms(&mut self, mouse: Point2<f32>, cursor_size: f32, exposure: f32) {
        self.uniforms.translate = [self.position.x, self.position.y];
        self.uniforms.view_size = [self.view_size.x, self.view_size.y];
        self.uniforms.mouse = [mouse.x, mouse.y];
        self.uniforms.cursor_size = cursor_size;
        self.uniforms.time = self.start_time.elapsed().as_secs_f32();
        self.uniforms.exposure = exposure;
    }

    pub fn update_lights(&mut self, queue: &mut wgpu::Queue, lights: &Vec<LightData>) {
        queue.write_buffer(&self.lights_buffer, 0, bytemuck::cast_slice(lights));
        queue.write_buffer(&self.lights_config_buffer, 0, bytemuck::cast_slice(&[LightsConfig { num_lights: lights.len() as u32 }]));
    }

    pub fn render(&mut self, device: &wgpu::Device, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
        self.light_map_renderer.render(device, queue, encoder, &self.uniform_bind_group, &self.sdf_bind_group, &self.lights_bind_group);
        {
            // Denoising and diffuse lighting pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Denoising and diffuse lighting pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view: &self.color_texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: 1.0,
                            }),
                            store: true,
                        }
                    }
                ],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.set_bind_group(0, &self.lightmap_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        self.taa.render(device, queue, encoder, &self.uniform_bind_group, &self.color_bind_group);
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
            render_pass.set_bind_group(1, &self.sdf_bind_group, &[]);
            render_pass.set_bind_group(2, &self.taa.output_bind_group(), &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}