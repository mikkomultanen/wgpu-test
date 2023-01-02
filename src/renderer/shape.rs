#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShapeData {
    pub data0: [u32; 4],
    pub data1: [f32; 4],
}

impl Default for ShapeData {
    fn default() -> Self {
        Self {
            data0: [0, 0, 0, 0],
            data1: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

trait RgbExt {
    fn to_u32(&self) -> u32;
}

impl RgbExt for [f32; 3] {
    fn to_u32(&self) -> u32 {
       u32::from_le_bytes([
            egui::color::gamma_u8_from_linear_f32(self[0]),
            egui::color::gamma_u8_from_linear_f32(self[1]),
            egui::color::gamma_u8_from_linear_f32(self[2]),
            0u8,
            ])
        
        /*let rgbe8pixel = image::codecs::hdr::to_rgbe8((*self).into());
        u32::from_le_bytes([rgbe8pixel.c[0], rgbe8pixel.c[1], rgbe8pixel.c[2], rgbe8pixel.e])*/
    }
} 

impl ShapeData {
    pub fn sphere() -> Self {
        Self {
            data0: [0, 0, 0, 0],
            data1: [0., 0., 0., 0.],
        }
    }

    pub fn update_sphere(&mut self, color: [f32; 3], metallic: f32, roughness: f32, position: cgmath::Vector3<f32>, radius: f32) {
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([egui::color::linear_u8_from_linear_f32(metallic), egui::color::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position.extend(radius).into();
}
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShapesConfig {
    pub num_shapes: u32,
}

impl Default for ShapesConfig {
    fn default() -> Self {
        Self {
            num_shapes: 0,
        }
    }
    
}