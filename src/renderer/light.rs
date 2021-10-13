#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LightData {
    pub color: [f32; 4],
    pub position: [f32; 2],
    pub radius: f32,
    pub range: f32,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 0.0],
            position: [0.0, 0.0],
            radius: 0.0,
            range: 1.0,
        }
    }
}

impl LightData {
    pub fn new(color: [f32; 3], position: [f32; 2], radius: f32, range: f32) -> Self {
        Self {
            color: [color[0], color[1], color[2], 0.0],
            position,
            radius,
            range,
        }
    }

    pub fn update(&mut self, color: [f32; 3], position: [f32; 2], radius: f32, range: f32) {
        self.color = [color[0], color[1], color[2], 0.0];
        self.position = position;
        self.radius = radius;
        self.range = range;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LightsConfig {
    pub num_lights: u32,
}

impl Default for LightsConfig {
    fn default() -> Self {
        Self {
            num_lights: 0,
        }
    }
    
}