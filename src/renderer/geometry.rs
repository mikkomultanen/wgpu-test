use cgmath::*;

use super::{texture, shape::ShapeData};

pub struct GeometryRenderer {
    pub diffuse: texture::Texture,
    pub normals_metallic_and_roughness: texture::Texture,
    pub depth: texture::Texture,
    terrain_pipeline: wgpu::RenderPipeline,
    shape_pipeline: wgpu::RenderPipeline,
}

const DIFFUSE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NORMALS_SPECULAR_AND_ROUGHNESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl GeometryRenderer {
    pub fn new(
        resolution: Vector2<u32>,
        device: &wgpu::Device,
        uniform_bind_group_layout: &wgpu::BindGroupLayout,
        sdf_bind_group_layout: &wgpu::BindGroupLayout,
        shapes_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let diffuse = texture::Texture::new_intermediate(device, resolution, DIFFUSE_FORMAT);
        let normals_metallic_and_roughness = texture::Texture::new_intermediate(
            device,
            resolution,
            NORMALS_SPECULAR_AND_ROUGHNESS_FORMAT,
        );
        let depth = texture::Texture::new_intermediate4(device, resolution, DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);

        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("geometry_terrain.wgsl").into()),
        });

        let terrain_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Render Pipeline Layout"),
                bind_group_layouts: &[
                    uniform_bind_group_layout,
                    sdf_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain Render Pipeline"),
            layout: Some(&terrain_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &terrain_shader,
                entry_point: "main_vert",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &terrain_shader,
                entry_point: "main_frag",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: DIFFUSE_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMALS_SPECULAR_AND_ROUGHNESS_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shape shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("geometry_shape.wgsl").into()),
        });

        let shape_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shape Render Pipeline Layout"),
                bind_group_layouts: &[
                    uniform_bind_group_layout,
                    shapes_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shape Render Pipeline"),
            layout: Some(&shape_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: "main_vert",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: "main_frag",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: DIFFUSE_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: NORMALS_SPECULAR_AND_ROUGHNESS_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        return Self {
            diffuse,
            normals_metallic_and_roughness,
            depth,
            terrain_pipeline,
            shape_pipeline,
        };
    }

    pub fn resize(&mut self, resolution: Vector2<u32>, device: &wgpu::Device) {
        self.diffuse = texture::Texture::new_intermediate(device, resolution, DIFFUSE_FORMAT);
        self.normals_metallic_and_roughness = texture::Texture::new_intermediate(
            device,
            resolution,
            NORMALS_SPECULAR_AND_ROUGHNESS_FORMAT,
        );
        self.depth = texture::Texture::new_intermediate4(device, resolution, DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
    }

    pub fn render(
        &mut self,
        _device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uniform_bind_group: &wgpu::BindGroup,
        sdf_bind_group: &wgpu::BindGroup,
        shapes_bind_group: &wgpu::BindGroup,
        shapes: &Vec<ShapeData>,
    ) {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Geometry pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.diffuse.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.normals_metallic_and_roughness.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: 0.,
                            }),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_bind_group(0, uniform_bind_group, &[]);
            render_pass.set_bind_group(1, sdf_bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            render_pass.set_pipeline(&self.shape_pipeline);
            render_pass.set_bind_group(0, uniform_bind_group, &[]);
            render_pass.set_bind_group(1, shapes_bind_group, &[]);
            render_pass.draw(0..4, 0..(shapes.len() as u32));
        }
    }
}
