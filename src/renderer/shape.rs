#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShapeData {
    pub data0: [u32; 4],
    pub data1: [f32; 4],
    pub data2: [f32; 4],
}

impl Default for ShapeData {
    fn default() -> Self {
        Self {
            data0: [0; 4],
            data1: [0.0; 4],
            data2: [0.0; 4],
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

enum Shape {
    Sphere = 0,
    RoundedCone = 1,
}

impl ShapeData {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn update_sphere(&mut self, 
        position: cgmath::Vector3<f32>, radius: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = Shape::Sphere as u32;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([egui::color::linear_u8_from_linear_f32(metallic), egui::color::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position.extend(radius).into();
    }

    pub fn update_rounded_cone(&mut self, 
        position_a: cgmath::Vector3<f32>, radius_a: f32,
        position_b: cgmath::Vector3<f32>, radius_b: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = Shape::RoundedCone as u32;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([egui::color::linear_u8_from_linear_f32(metallic), egui::color::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position_a.extend(radius_a).into();
        self.data2 = position_b.extend(radius_b).into();
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