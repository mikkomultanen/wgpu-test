use cgmath::*;
use std::time::Instant;
use wgpu::util::DeviceExt;

use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniforms {
    pub translate: [f32; 2],
    pub view_size: [f32; 2],
    pub world_size: [f32; 2],
    pub inv_world_size: [f32; 2],
    pub mouse: [f32; 2],
    pub cursor_size: f32,
    pub time: f32,
}

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms {
            translate: [0.0, 0.0],
            view_size: [1.0, 1.0],
            world_size: [1.0, 1.0], 
            inv_world_size: [1.0, 1.0],
            mouse: [0.0, 0.0],
            cursor_size: 0.0,
            time: 0.0,
        }
    }
}

pub struct Renderer {
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    sdf_bind_group: wgpu::BindGroup,
    blue_noise_textures: Vec<wgpu::BindGroup>,
    blue_noise_index: usize,
    lightmap_view: wgpu::TextureView,
    lightmap_sampler: wgpu::Sampler,
    lightmap_bind_group_layout: wgpu::BindGroupLayout,
    lightmap_bind_group: wgpu::BindGroup,
    lightmap_pipeline: wgpu::RenderPipeline,
    render_pipeline: wgpu::RenderPipeline,
    start_time: Instant,
    pub position: Point2<f32>,
    pub view_size: Vector2<f32>,
}

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl Renderer {
    pub fn new(resolution: Vector2<f32>, world_size: Vector2<f32>, device: &wgpu::Device, queue: &mut wgpu::Queue, sdf_view: &wgpu::TextureView, sdf_sampler: &wgpu::Sampler, surface_format: &wgpu::TextureFormat) -> Self {
        let mut uniforms = Uniforms::default();
        uniforms.view_size = [world_size.x, world_size.y];
        uniforms.world_size = [world_size.x, world_size.y];
        uniforms.inv_world_size = [1.0 / world_size.x, 1.0 / world_size.y];

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

        let sdf_texture_bind_group_layout = device.create_bind_group_layout(
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
                layout: &sdf_texture_bind_group_layout,
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

        let blue_noise_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                ],
                label: None,
            }
        );

        let blue_noise_textures = (0..64).into_iter().map(|i| {
            let img = image::open(format!("assets/blue_noise/LDR_RGBA_{}.png", i)).unwrap();
            let texture = texture::Texture::from_image(device, queue, &img, None);
            return device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &blue_noise_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture.view),
                        },
                    ],
                    label: None,
                }
            );
        }).collect();

        let lightmap_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: (resolution.x.ceil() as u32).max(16).min(4096),
                height: (resolution.y.ceil() as u32).max(16).min(4096),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Renderer result"),
        });
        let lightmap_view = lightmap_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
                label: Some("renderer_texture_bind_group_layout"),
            }
        );

        let lightmap_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &lightmap_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&lightmap_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&lightmap_sampler),
                    }
                ],
                label: Some("renderer_view_bind_group"),
            }
        );

        let lightmap_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Lightmap shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("renderer.wgsl").into()),
        });

        let lightmap_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &sdf_texture_bind_group_layout, &blue_noise_bind_group_layout],
                push_constant_ranges: &[],
            });

        let lightmap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&lightmap_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &lightmap_shader,
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &lightmap_shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: TEXTURE_FORMAT,
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

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &sdf_texture_bind_group_layout, &lightmap_bind_group_layout],
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
        let view_size = Vector2::new(world_size.x, world_size.y);

        return Self {
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            sdf_bind_group,
            blue_noise_textures,
            blue_noise_index: 0,
            lightmap_view,
            lightmap_sampler,
            lightmap_bind_group_layout,
            lightmap_bind_group,
            lightmap_pipeline,
            render_pipeline,
            start_time,
            position,
            view_size,
        }
    }

    pub fn resize(&mut self, resolution: Vector2<f32>, device: &wgpu::Device) {
        let lightmap_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: (resolution.x.ceil() as u32).max(16).min(4096),
                height: (resolution.y.ceil() as u32).max(16).min(4096),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Renderer result"),
        });
        self.lightmap_view = lightmap_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.lightmap_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &self.lightmap_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.lightmap_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.lightmap_sampler),
                    }
                ],
                label: Some("renderer_view_bind_group"),
            }
        );

        self.view_size.x = self.view_size.y * resolution.x / resolution.y;
    }

    pub fn update_uniforms(&mut self, mouse: Point2<f32>, cursor_size: f32) {
        self.uniforms.translate = [self.position.x, self.position.y];
        self.uniforms.view_size = [self.view_size.x, self.view_size.y];
        self.uniforms.mouse = [mouse.x, mouse.y];
        self.uniforms.cursor_size = cursor_size;
        self.uniforms.time = self.start_time.elapsed().as_secs_f32();
    }

    pub fn render(&mut self, queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.blue_noise_index = (self.blue_noise_index + 1) % self.blue_noise_textures.len();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view: &self.lightmap_view,
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
            render_pass.set_pipeline(&self.lightmap_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.sdf_bind_group, &[]);
            render_pass.set_bind_group(2, &self.blue_noise_textures[self.blue_noise_index], &[]);
            render_pass.draw(0..3, 0..1);
        }

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
            render_pass.set_bind_group(2, &self.lightmap_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}