use glam::*;
use wgpu::{util::DeviceExt, PipelineCompilationOptions};

use crate::renderer::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniforms {
    pub world_pos: [f32; 2],
    pub world_size: [f32; 2],
    pub inv_world_size: [f32; 2],
    pub radius: f32,
    pub smoothness: f32,
}

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms {
            world_pos: [0.0, 0.0],
            world_size: [1.0, 1.0],
            inv_world_size: [1.0, 1.0],
            radius: 10.0,
            smoothness: 1.0,
        }
    }
}

pub struct SDF {
    textures: [texture::Texture; 2],
    pipeline: wgpu::RenderPipeline,
    subtract_pipeline: wgpu::RenderPipeline,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_index: usize,
    pub sdf_bind_group_layout: wgpu::BindGroupLayout,
    sdf_bind_groups: [wgpu::BindGroup; 2],
}

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

impl SDF {
    pub fn new(size: UVec2, world_size: Vec2, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let textures = [
            texture::Texture::new_intermediate(device, size, TEXTURE_FORMAT),
            texture::Texture::new_intermediate(device, size, TEXTURE_FORMAT),
        ];

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Init SDF"),
        });

        {
            let l = Vec2 {
                x: size.x as f32,
                y: size.y as f32,
            }.length() as f64;
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &textures[0].view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: l,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });                    
        }
    
        queue.submit(std::iter::once(encoder.finish()));

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SDF"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
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
                ],
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("sdf_texture_bind_group_layout"),
            }
        );
    
        let sdf_bind_groups = [
            Self::create_output_bind_group(device, &sdf_bind_group_layout, &textures[0].view, &sampler),
            Self::create_output_bind_group(device, &sdf_bind_group_layout, &textures[1].view, &sampler),
        ];
    
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SDF"),
            bind_group_layouts: &[&bind_group_layout, &sdf_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut uniforms = Uniforms::default();
        uniforms.world_size = [world_size.x, world_size.y];
        uniforms.inv_world_size = [1. / world_size.x, 1. / world_size.y];

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SDF Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sdf.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main_vert",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main_frag",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let subtract_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SDF"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main_vert",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main_frag_subtract",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        return Self {
            textures,
            pipeline,
            subtract_pipeline,
            uniforms,
            uniform_buffer,
            texture_index: 0,
            uniform_bind_group,
            sdf_bind_group_layout,
            sdf_bind_groups,
        }
    }

    fn create_output_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        })
    }

    pub fn add(&mut self, mouse: Vec2, cursor_size: f32, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {
        self.uniforms.world_pos = mouse.into();
        self.uniforms.radius = 0.25 * cursor_size;
        self.uniforms.smoothness = 0.25 * cursor_size;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        let sdf_bind_group = &mut self.sdf_bind_groups[self.texture_index];
        self.texture_index = (self.texture_index + 1) % 2;
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.textures[self.texture_index].view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &sdf_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }

    pub fn subtract(&mut self, mouse: Vec2, cursor_size: f32, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {
        self.uniforms.world_pos = [mouse.x, mouse.y];
        self.uniforms.radius = 0.25 * cursor_size;
        self.uniforms.smoothness = 0.25 * cursor_size;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        let sdf_bind_group = &mut self.sdf_bind_groups[self.texture_index];
        self.texture_index = (self.texture_index + 1) % 2;
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.textures[self.texture_index].view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.subtract_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &sdf_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }

    pub fn output_bind_group(&self) -> &wgpu::BindGroup {
        &self.sdf_bind_groups[self.texture_index]
    }
}