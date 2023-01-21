use cgmath::{Vector3, Point3, EuclideanSpace};

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
            ecolor::gamma_u8_from_linear_f32(self[0]),
            ecolor::gamma_u8_from_linear_f32(self[1]),
            ecolor::gamma_u8_from_linear_f32(self[2]),
            0u8,
            ])
        
        /*let rgbe8pixel = image::codecs::hdr::to_rgbe8((*self).into());
        u32::from_le_bytes([rgbe8pixel.c[0], rgbe8pixel.c[1], rgbe8pixel.c[2], rgbe8pixel.e])*/
    }
}

trait TranslateXYZ {
    fn translate(&mut self, translate: [f32; 3]);
}

impl TranslateXYZ for [f32; 4] {
    fn translate(&mut self, translate: [f32; 3]) {
        self[0] += translate[0];
        self[1] += translate[1];
        self[2] += translate[2];
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
        position: Point3<f32>, radius: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = Shape::Sphere as u32;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([ecolor::linear_u8_from_linear_f32(metallic), ecolor::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position.to_vec().extend(radius).into();
    }

    pub fn update_rounded_cone(&mut self, 
        position_a: Point3<f32>, radius_a: f32,
        position_b: Point3<f32>, radius_b: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = Shape::RoundedCone as u32;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([ecolor::linear_u8_from_linear_f32(metallic), ecolor::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position_a.to_vec().extend(radius_a).into();
        self.data2 = position_b.to_vec().extend(radius_b).into();
    }

    pub fn translate(&mut self, translate: Vector3<f32>) {
        self.data1.translate(translate.into());
        self.data2.translate(translate.into());
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