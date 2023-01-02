use cgmath::*;

use super::texture;

pub struct LightMapRenderer {
    blue_noise_textures: Vec<wgpu::BindGroup>,
    blue_noise_index: usize,
    pub lightmap_view: wgpu::TextureView,
    lightmap_pipeline: wgpu::RenderPipeline,
}

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

impl LightMapRenderer {
    pub fn new(resolution: Vector2<u32>, device: &wgpu::Device, queue: &mut wgpu::Queue, uniform_bind_group_layout: &wgpu::BindGroupLayout, sdf_bind_group_layout: &wgpu::BindGroupLayout, lights_bind_group_layout: &wgpu::BindGroupLayout, shapes_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
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
                width: resolution.x,
                height: resolution.y,
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

        let lightmap_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Lightmap shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("light_map.wgsl").into()),
        });

        let lightmap_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Lightmap Render Pipeline Layout"),
                bind_group_layouts: &[uniform_bind_group_layout, sdf_bind_group_layout, lights_bind_group_layout, shapes_bind_group_layout, &blue_noise_bind_group_layout],
                push_constant_ranges: &[],
            });

        let lightmap_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Lightmap Render Pipeline"),
            layout: Some(&lightmap_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &lightmap_shader,
                entry_point: "main_vert",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &lightmap_shader,
                entry_point: "main_frag_pbr",
                targets: &[Some(wgpu::ColorTargetState {
                    format: TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
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
            blue_noise_textures,
            blue_noise_index: 0,
            lightmap_view,
            lightmap_pipeline,
        }
    }

    pub fn resize(&mut self, resolution: Vector2<u32>, device: &wgpu::Device) {
        let lightmap_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: resolution.x,
                height: resolution.y,
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
    }

    pub fn render(&mut self, _device: &wgpu::Device, _queue: &mut wgpu::Queue, encoder: &mut wgpu::CommandEncoder, uniform_bind_group: &wgpu::BindGroup, sdf_bind_group: &wgpu::BindGroup, lights_bind_group: &wgpu::BindGroup, shapes_bind_group: &wgpu::BindGroup) {
        self.blue_noise_index = (self.blue_noise_index + 1) % self.blue_noise_textures.len();
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
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
                    })
                ],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.lightmap_pipeline);
            render_pass.set_bind_group(0, uniform_bind_group, &[]);
            render_pass.set_bind_group(1, sdf_bind_group, &[]);
            render_pass.set_bind_group(2, lights_bind_group, &[]);
            render_pass.set_bind_group(3, shapes_bind_group, &[]);
            render_pass.set_bind_group(4, &self.blue_noise_textures[self.blue_noise_index], &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}