use crate::rasterizer::{WORLD_HEIGHT, WORLD_WIDTH};

pub struct Camera {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,

    pub target_pan_x: f32,
    pub target_pan_y: f32,
    pub target_zoom: f32,

    pub min_zoom: f32,
    pub screen_width: f32,
    pub screen_height: f32,

    pub is_animating: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            target_pan_x: 0.0,
            target_pan_y: 0.0,
            target_zoom: 1.0,
            min_zoom: 0.8,
            screen_width: 1500.0,
            screen_height: 900.0,
            is_animating: false,
        }
    }
}

impl Camera {
    pub fn fit_to_screen(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;

        let scale_x = width / WORLD_WIDTH;
        let scale_y = height / WORLD_HEIGHT;
        let fit_zoom = scale_x.min(scale_y);

        let fit_pan_x = (width - WORLD_WIDTH * fit_zoom) * 0.5;
        let fit_pan_y = (height - WORLD_HEIGHT * fit_zoom) * 0.5;

        self.min_zoom = fit_zoom;
        self.zoom = fit_zoom;
        self.target_zoom = fit_zoom;
        self.pan_x = fit_pan_x;
        self.target_pan_x = fit_pan_x;
        self.pan_y = fit_pan_y;
        self.target_pan_y = fit_pan_y;
        self.is_animating = false;
    }

    pub fn zoom_at(&mut self, screen_x: f32, screen_y: f32, factor: f32) {
        let new_zoom = (self.target_zoom * factor).clamp(self.min_zoom, 120.0);
        let world_pt = self.screen_to_world(screen_x, screen_y);

        self.target_zoom = new_zoom;
        self.target_pan_x = screen_x - world_pt[0] * new_zoom;
        self.target_pan_y = screen_y - world_pt[1] * new_zoom;
        self.is_animating = true;
    }

    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.target_pan_x += dx;
        self.target_pan_y += dy;
        self.pan_x = self.target_pan_x;
        self.pan_y = self.target_pan_y;
        self.is_animating = false;
    }

    pub fn jump_to(&mut self, world_x: f32, world_y: f32, target_zoom: f32) {
        let z = target_zoom.clamp(self.min_zoom, 120.0);
        self.target_zoom = z;
        self.target_pan_x = self.screen_width * 0.5 - world_x * z;
        self.target_pan_y = self.screen_height * 0.5 - world_y * z;
        self.is_animating = true;
    }

    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> [f32; 2] {
        [
            (screen_x - self.pan_x) / self.zoom,
            (screen_y - self.pan_y) / self.zoom,
        ]
    }

    pub fn update(&mut self) {
        if !self.is_animating {
            return;
        }

        let lerp = 0.25;
        let diff_z = self.target_zoom - self.zoom;
        let diff_x = self.target_pan_x - self.pan_x;
        let diff_y = self.target_pan_y - self.pan_y;

        if diff_z.abs() < 0.001 && diff_x.abs() < 0.5 && diff_y.abs() < 0.5 {
            self.zoom = self.target_zoom;
            self.pan_x = self.target_pan_x;
            self.pan_y = self.target_pan_y;
            self.is_animating = false;
            return;
        }

        self.zoom += diff_z * lerp;
        self.pan_x += diff_x * lerp;
        self.pan_y += diff_y * lerp;
    }
}
