use cgmath::Vector2;
use wgpu::PipelineCompilationOptions;

use super::texture;

pub struct TAA {
    output_resolution: Vector2<u32>,
    textures: [texture::Texture; 2],
    taa_sampler: wgpu::Sampler,
    taa_bind_group_layout: wgpu::BindGroupLayout,
    taa_bind_groups: [wgpu::BindGroup; 2],
    output_sampler: wgpu::Sampler,
    output_bind_groups: [wgpu::BindGroup; 2],
    history_texture_index: usize,
    taa_pipeline: wgpu::ComputePipeline,
}

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

impl TAA {
    pub fn new(resolution: Vector2<u32>, device: &wgpu::Device, _queue: &wgpu::Queue, uniform_bind_group_layout: &wgpu::BindGroupLayout, color_bind_group_layout: &wgpu::BindGroupLayout, output_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let textures = [
            texture::Texture::new_intermediate(device, resolution, TEXTURE_FORMAT),
            texture::Texture::new_intermediate(device, resolution, TEXTURE_FORMAT),
        ];
        
        let taa_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let taa_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: TEXTURE_FORMAT,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
                label: None,
            }
        );

        let taa_bind_groups = [
            Self::create_taa_bind_group(device, &taa_bind_group_layout, &textures[0].view, &taa_sampler, &textures[1].view),
            Self::create_taa_bind_group(device, &taa_bind_group_layout, &textures[1].view, &taa_sampler, &textures[0].view),
        ];

        let output_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let output_bind_groups = [
            Self::create_output_bind_group(device, output_bind_group_layout, &textures[0].view, &output_sampler),
            Self::create_output_bind_group(device, output_bind_group_layout, &textures[1].view, &output_sampler),
        ];

        let taa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("taa.wgsl").into()),
        });

        let taa_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&uniform_bind_group_layout, &color_bind_group_layout, &taa_bind_group_layout],
                push_constant_ranges: &[],
            });

        let taa_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("TAA compute pipeline"),
            layout: Some(&taa_pipeline_layout),
            module: &taa_shader,
            entry_point: "main",
            compilation_options: PipelineCompilationOptions::default(),
        });

        Self {
            output_resolution: resolution,
            textures,
            taa_sampler,
            taa_bind_group_layout,
            taa_bind_groups,
            output_sampler,
            output_bind_groups,
            history_texture_index: 0,
            taa_pipeline,
        }
    }

    fn create_taa_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, history_view: &wgpu::TextureView, sampler: &wgpu::Sampler, output_view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(history_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(output_view),
                },
            ],
            label: None,
        })
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

    pub fn resize(&mut self, resolution: Vector2<u32>) {
        self.output_resolution = resolution;
    }

    pub fn render(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, uniform_bind_group: &wgpu::BindGroup, color_bind_group: &wgpu::BindGroup, output_bind_group_layout: &wgpu::BindGroupLayout) {
        let taa_bind_group = &mut self.taa_bind_groups[self.history_texture_index];
        self.history_texture_index = (self.history_texture_index + 1) % 2;
        let output_texture_size = self.textures[self.history_texture_index].size;
        if output_texture_size.width != self.output_resolution.x || output_texture_size.height != self.output_resolution.y {
            self.textures[self.history_texture_index] = texture::Texture::new_intermediate(device, self.output_resolution, TEXTURE_FORMAT);
            let history_view = &self.textures[(self.history_texture_index + 1) % 2].view;
            let output_view = &self.textures[self.history_texture_index].view;
            *taa_bind_group = Self::create_taa_bind_group(device, &self.taa_bind_group_layout, history_view, &self.taa_sampler, output_view);
            self.output_bind_groups[self.history_texture_index] = Self::create_output_bind_group(device, output_bind_group_layout, output_view, &self.output_sampler);
        }

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None, });
            compute_pass.set_pipeline(&self.taa_pipeline);
            compute_pass.set_bind_group(0, uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, color_bind_group, &[]);
            compute_pass.set_bind_group(2, taa_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                (self.output_resolution.x as f32 / 16.).ceil() as u32,
                (self.output_resolution.y as f32 / 16.).ceil() as u32,
                1
            );
        }
    }

    pub fn output_bind_group(&self) -> &wgpu::BindGroup {
        &self.output_bind_groups[self.history_texture_index]
    }
}