use super::texture;

pub struct BLIT {
    color_texture_size: wgpu::Extent3d,
    output_sampler: wgpu::Sampler,
    output_bind_group: wgpu::BindGroup,
}

impl BLIT {
    pub fn new(device: &wgpu::Device, color_texture: &texture::Texture, output_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let output_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let output_bind_group = Self::create_output_bind_group(device, output_bind_group_layout, &color_texture.view, &output_sampler);

        Self {
            color_texture_size: color_texture.size,
            output_sampler,
            output_bind_group,
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

    pub fn update_output_bind_group(&mut self, device: &wgpu::Device, color_texture: &texture::Texture, output_bind_group_layout: &wgpu::BindGroupLayout) {
        if self.color_texture_size != color_texture.size {
            self.output_bind_group = Self::create_output_bind_group(device, output_bind_group_layout, &color_texture.view, &self.output_sampler);
        }
    }

    pub fn output_bind_group(&self) -> &wgpu::BindGroup {
        &self.output_bind_group
    }
}