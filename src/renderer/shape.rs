use bvh::{aabb::{AABB, Bounded}, bounding_hierarchy::BHShape};
use glam::Vec3;

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

const SHAPE_SPHERE: u32 = 0;
const SHAPE_ROUNDED_CONE: u32 = 1;

impl ShapeData {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn update_sphere(&mut self, 
        position: Vec3, radius: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = SHAPE_SPHERE;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([ecolor::linear_u8_from_linear_f32(metallic), ecolor::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position.extend(radius).into();
    }

    pub fn update_rounded_cone(&mut self, 
        position_a: Vec3, radius_a: f32,
        position_b: Vec3, radius_b: f32,
        color: [f32; 3], metallic: f32, roughness: f32, 
    ) {
        self.data0[0] = SHAPE_ROUNDED_CONE;
        self.data0[1] = color.to_u32();
        self.data0[2] = u32::from_le_bytes([ecolor::linear_u8_from_linear_f32(metallic), ecolor::linear_u8_from_linear_f32(roughness), 0u8, 0u8]);
        self.data1 = position_a.extend(radius_a).into();
        self.data2 = position_b.extend(radius_b).into();
    }

    pub fn translate(&mut self, translate: Vec3) {
        self.data1.translate(translate.into());
        self.data2.translate(translate.into());
    }
}

impl Bounded for ShapeData {
    fn aabb(&self) -> AABB {
        match self.data0[0] {
            SHAPE_SPHERE => {
                let position = bvh::Point3::from_slice(&self.data1[0..3]);
                let radius = self.data1[3];
                AABB::with_bounds(position - radius, position + radius)
            }
            SHAPE_ROUNDED_CONE => {
                let position_a = bvh::Point3::from_slice(&self.data1[0..3]);
                let radius_a = self.data1[3];
                let position_b= bvh::Point3::from_slice(&self.data2[0..3]);
                let radius_b = self.data2[3];
                AABB::with_bounds(
                    (position_a - radius_a).min(position_b - radius_b),
                    (position_a + radius_a).max(position_b + radius_b),
                )
            }
            _ => panic!("Not possible!!!")
        }
    }
}

impl BHShape for ShapeData {
    fn set_bh_node_index(&mut self, index: usize) {
        self.data0[3] = index as u32;
    }

    fn bh_node_index(&self) -> usize {
        self.data0[3] as usize
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShapeBVHNode {
    pub aabb_pos: [f32; 3],
    pub entry: i32,
    pub aabb_rad: [f32; 3],
    pub exit: i32,
}

impl Default for ShapeBVHNode {
    fn default() -> Self {
        Self {
            aabb_pos: [0.0; 3],
            entry: 0,
            aabb_rad: [0.0; 3],
            exit: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ShapesConfig {
    pub num_shapes: u32,
    pub num_bvh_nodes: u32,
}

impl Default for ShapesConfig {
    fn default() -> Self {
        Self {
            num_shapes: 0,
            num_bvh_nodes: 0,
        }
    }
    
}