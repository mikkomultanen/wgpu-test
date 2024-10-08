mod geometry;
mod light_map;
pub mod texture;
mod taa;
mod blit_sampler;
pub mod light;
pub mod shape;

use glam::*;
use wgpu::PipelineCompilationOptions;
use std::time::Instant;
use wgpu::util::DeviceExt;

use light_map::LightMapRenderer;

use crate::renderer::light::{LightData, LightsConfig};
use crate::renderer::shape::{ShapeBVHNode, ShapeData, ShapesConfig};
use crate::sdf::SDF;

use self::geometry::GeometryRenderer;

pub const MAX_LIGHTS: usize = 1024;
pub const MAX_SHAPES: usize = 4096;
const NUM_SUBPIXEL_JITTER_SAMPLES: usize = 16;

fn halton(base: usize, index: usize) -> f32 {
    let mut f = 1.;
    let mut r = 0.;
    let mut i = index;

    while i > 0
    {
        f = f / base as f32;
        r = r + f * (i % base) as f32;
        i = i / base;
    }
    return r;
}

#[derive(Debug, PartialEq)]
pub enum Upsampler {
    TAA,
    BLIT,
}

enum UpsamplerCell {
    TAA(taa::TAA),
    BLIT(blit_sampler::BLIT),
}

impl UpsamplerCell {
    pub fn resize(&mut self, resolution: UVec2) {
        match self {
            Self::TAA(taa) => taa.resize(resolution),
            Self::BLIT(_) => {},
        }
    }

    pub fn output_bind_group(&self) -> &wgpu::BindGroup {
        match self {
            Self::TAA(taa) => taa.output_bind_group(),
            Self::BLIT(blit) => blit.output_bind_group(),
        }
    }

    pub fn upsampler(&self) -> Upsampler {
        match self {
            Self::TAA(_) => Upsampler::TAA,
            Self::BLIT(_) => Upsampler::BLIT,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniforms {
    pub translate: [f32; 2],
    pub view_size: [f32; 2],
    pub world_size: [f32; 2],
    pub inv_world_size: [f32; 2],
    pub pixel_size: [f32; 2],
    pub sub_pixel_jitter: [f32; 2],
    pub mouse: [f32; 2],
    pub cursor_size: f32,
    pub time: f32,
    pub exposure: f32,
    pub dummy: f32,
}

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms {
            translate: [0.0, 0.0],
            view_size: [1.0, 1.0],
            world_size: [1.0, 1.0], 
            inv_world_size: [1.0, 1.0],
            pixel_size: [1.0, 1.0],
            sub_pixel_jitter: [0.0, 0.0],
            mouse: [0.0, 0.0],
            cursor_size: 0.0,
            time: 0.0,
            exposure: 1.0,
            dummy: 0.0,
        }
    }
}

pub struct Renderer {
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_bind_group: wgpu::BindGroup,
    lights_buffer: wgpu::Buffer,
    lights_config_buffer: wgpu::Buffer,
    lights_bind_group: wgpu::BindGroup,
    shapes_buffer: wgpu::Buffer,
    bvh: Vec<ShapeBVHNode>,
    bvh_buffer: wgpu::Buffer,
    shapes_config_buffer: wgpu::Buffer,
    shapes_bind_group: wgpu::BindGroup,
    geometry_renderer: GeometryRenderer,
    geometry_bind_group_layout: wgpu::BindGroupLayout,
    geometry_bind_group: wgpu::BindGroup,
    light_map_renderer: LightMapRenderer,
    lightmap_sampler: wgpu::Sampler,    
    lightmap_bind_group_layout: wgpu::BindGroupLayout,
    lightmap_bind_group: wgpu::BindGroup,
    color_texture: texture::Texture,
    blit_pipeline: wgpu::RenderPipeline,
    color_bind_group_layout: wgpu::BindGroupLayout,
    color_bind_group: wgpu::BindGroup,
    subpixel_jitter_samples: Vec<[f32; 2]>,
    subpixel_jitter_index: usize,
    upsampler_output_bind_group_layout: wgpu::BindGroupLayout,
    upsampler: UpsamplerCell,
    render_pipeline: wgpu::RenderPipeline,
    start_time: Instant,
    render_resolution: UVec2, 
    output_resolution: UVec2,
    pub position: Vec2,
    pub view_size: Vec2,
}

const COLOR_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

impl Renderer {
    pub fn new(render_resolution: UVec2, output_resolution: UVec2, world_size: Vec2, device: &wgpu::Device, queue: &wgpu::Queue, sdf: &SDF, surface_format: &wgpu::TextureFormat) -> Self {
        let mut view_size = Vec2::new(world_size.x / 4., world_size.y / 4.);
        view_size.x = view_size.y * output_resolution.x as f32 / output_resolution.y as f32;

        let mut uniforms = Uniforms::default();
        uniforms.view_size = [view_size.x, view_size.y];
        uniforms.world_size = [world_size.x, world_size.y];
        uniforms.inv_world_size = [1.0 / world_size.x, 1.0 / world_size.y];
        uniforms.pixel_size = [view_size.x / render_resolution.x as f32, view_size.y / render_resolution.y as f32];

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
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
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

        let initial_shapes_data = vec![ShapeData::default(); MAX_SHAPES];
        let initial_bvh_data = vec![ShapeBVHNode::default(); 4 * MAX_SHAPES];
        let shapes_config = ShapesConfig { num_shapes: 0, num_bvh_nodes: 0, };

        let shapes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&initial_shapes_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        });
        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&initial_bvh_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
        });

        let shapes_config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[shapes_config]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let shapes_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                },
            ]
        });

        let shapes_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shapes_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shapes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bvh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shapes_config_buffer.as_entire_binding(),
                },
            ],
            label: None,
        });

        let geometry_renderer = GeometryRenderer::new(render_resolution, device, &uniform_bind_group_layout, &sdf.sdf_bind_group_layout, &shapes_bind_group_layout);

        let geometry_bind_group_layout = device.create_bind_group_layout(
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                ],
                label: Some("geometry_bind_group_layout"),
            }
        );

        let geometry_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &geometry_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&geometry_renderer.diffuse.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&geometry_renderer.normals_metallic_and_roughness.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&geometry_renderer.depth.view),
                    },
                ],
                label: Some("geometry_bind_group"),
            }
        );

        let light_map_renderer = LightMapRenderer::new(render_resolution, device, queue, &uniform_bind_group_layout, &sdf.sdf_bind_group_layout, &lights_bind_group_layout, &shapes_bind_group_layout, &geometry_bind_group_layout);

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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
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
                entry_point: "main_vert",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: "main_frag",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_TEXTURE_FORMAT,
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

        let subpixel_jitter_samples = (0..NUM_SUBPIXEL_JITTER_SAMPLES)
            .map(|i| [halton(2, i + 1) - 0.5, halton(3, i + 1) - 0.5])
            .collect();

        let upsampler_output_bind_group_layout = device.create_bind_group_layout(
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
                label: None,
            }
        );
        let upsampler = UpsamplerCell::TAA(taa::TAA::new(output_resolution, device, queue, &uniform_bind_group_layout, &color_bind_group_layout, &upsampler_output_bind_group_layout));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("renderer.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Renderer Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &sdf.sdf_bind_group_layout, &upsampler_output_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Renderer Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                targets: &[Some((*surface_format).into())],
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

        let start_time = Instant::now();
        let position = Vec2::new(0., 0.);

        return Self {
            uniforms,
            uniform_buffer,
            uniform_bind_group_layout,
            uniform_bind_group,
            lights_buffer,
            lights_config_buffer,
            lights_bind_group,
            shapes_buffer,
            bvh: vec![],
            bvh_buffer,
            shapes_config_buffer,
            shapes_bind_group,
            geometry_renderer,
            geometry_bind_group_layout,
            geometry_bind_group,
            light_map_renderer,
            lightmap_sampler,
            lightmap_bind_group_layout,
            lightmap_bind_group,
            color_texture,
            blit_pipeline,
            color_bind_group_layout,
            color_bind_group,
            subpixel_jitter_samples,
            subpixel_jitter_index: 0,
            upsampler_output_bind_group_layout,
            upsampler,
            render_pipeline,
            start_time,
            render_resolution,
            output_resolution,
            position,
            view_size,
        }
    }

    pub fn resize_render_resolution(&mut self, render_resolution: UVec2, device: &wgpu::Device) {
        self.geometry_renderer.resize(render_resolution, device);
        self.geometry_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &self.geometry_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.geometry_renderer.diffuse.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.geometry_renderer.normals_metallic_and_roughness.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.geometry_renderer.depth.view),
                    },
                ],
                label: Some("geometry_bind_group"),
            }
        );
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
        match &mut self.upsampler {
            UpsamplerCell::TAA(_) => {},
            UpsamplerCell::BLIT(blit) => blit.update_output_bind_group(device, &self.color_texture, &self.upsampler_output_bind_group_layout)
        }

        self.render_resolution = render_resolution;
    }

    pub fn resize(&mut self, render_resolution: UVec2, output_resolution: UVec2, device: &wgpu::Device) {
        self.resize_render_resolution(render_resolution, device);

        self.upsampler.resize(output_resolution);

        self.output_resolution = output_resolution;
        self.view_size.x = self.view_size.y * output_resolution.x as f32 / output_resolution.y as f32;
    }

    pub fn update_uniforms(&mut self, mouse: Vec2, cursor_size: f32, exposure: f32) {
        self.uniforms.translate = [self.position.x, self.position.y];
        self.uniforms.view_size = [self.view_size.x, self.view_size.y];
        self.uniforms.pixel_size = [self.view_size.x / self.render_resolution.x as f32, self.view_size.y / self.render_resolution.y as f32];
        self.uniforms.sub_pixel_jitter = match self.upsampler.upsampler() {
            Upsampler::TAA => self.subpixel_jitter_samples[self.subpixel_jitter_index],
            Upsampler::BLIT => [0., 0.],
        };
        self.uniforms.mouse = [mouse.x, mouse.y];
        self.uniforms.cursor_size = cursor_size;
        self.uniforms.time = self.start_time.elapsed().as_secs_f32();
        self.uniforms.exposure = exposure;
    }

    pub fn update_lights(&mut self, queue: &wgpu::Queue, lights: &Vec<LightData>) {
        queue.write_buffer(&self.lights_buffer, 0, bytemuck::cast_slice(lights));
        queue.write_buffer(&self.lights_config_buffer, 0, bytemuck::cast_slice(&[LightsConfig { num_lights: lights.len() as u32 }]));
    }

    pub fn update_shapes(&mut self, queue: &wgpu::Queue, shapes: &mut Vec<ShapeData>) {
        self.bvh = bvh::bvh::BVH::build(shapes).flatten_custom(&|aabb, entry, exit, shape| ShapeBVHNode {
            aabb_pos: ((aabb.min + aabb.max) * 0.5).into(),
            entry: if entry == u32::max_value() { -(shape as i32) } else { entry as i32 },
            aabb_rad: ((aabb.max - aabb.min) * 0.5).into(),
            exit: exit as i32,
        });
        queue.write_buffer(&self.bvh_buffer, 0, bytemuck::cast_slice(&self.bvh));
        queue.write_buffer(&self.shapes_buffer, 0, bytemuck::cast_slice(shapes));
        queue.write_buffer(&self.shapes_config_buffer, 0, bytemuck::cast_slice(&[ShapesConfig { num_shapes: shapes.len() as u32, num_bvh_nodes: self.bvh.len() as u32 }]));
    }

    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, sdf: &SDF, shapes: &Vec<ShapeData>, view: &wgpu::TextureView) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
        self.geometry_renderer.render(device, encoder, &self.uniform_bind_group, sdf.output_bind_group(), &self.shapes_bind_group, shapes);
        self.light_map_renderer.render(device, queue, encoder, &self.uniform_bind_group, sdf.output_bind_group(), &self.lights_bind_group, &self.shapes_bind_group, &self.geometry_bind_group);
        {
            // Denoising and diffuse lighting pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Denoising and diffuse lighting pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.color_texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.blit_pipeline);
            render_pass.set_bind_group(0, &self.lightmap_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        match &mut self.upsampler {
            UpsamplerCell::TAA(taa) => {
                taa.render(device, queue, encoder, &self.uniform_bind_group, &self.color_bind_group, &self.upsampler_output_bind_group_layout);
            },
            UpsamplerCell::BLIT(_) => {},
        }
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, sdf.output_bind_group(), &[]);
            render_pass.set_bind_group(2, &self.upsampler.output_bind_group(), &[]);
            render_pass.draw(0..3, 0..1);
        }
        self.subpixel_jitter_index = (self.subpixel_jitter_index + 1) % self.subpixel_jitter_samples.len();
    }

    pub fn update_upsampler(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, upsampler: &Upsampler) {
        if *upsampler != self.upsampler.upsampler() {
            self.upsampler = match upsampler {
                Upsampler::TAA => UpsamplerCell::TAA(taa::TAA::new(self.output_resolution, device, queue, &self.uniform_bind_group_layout, &self.color_bind_group_layout, &self.upsampler_output_bind_group_layout)),
                Upsampler::BLIT => UpsamplerCell::BLIT(blit_sampler::BLIT::new(device, &self.color_texture, &self.upsampler_output_bind_group_layout)),
            }
        }
    }
}