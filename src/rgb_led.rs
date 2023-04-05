pub use rgb::{RGB8, RGBA8};

pub trait RGBABrightnessExt {
    fn update_channels(&self, rgb: &mut RGB8);
}

impl RGBABrightnessExt for RGBA8 {
    fn update_channels(&self, rgb: &mut RGB8) {
        let rel_brightness: f64 = self.a as f64 / 255.0;
        rgb.r = ((self.r as f64) * rel_brightness).round() as u8;
        rgb.g = ((self.g as f64) * rel_brightness).round() as u8;
        rgb.b = ((self.b as f64) * rel_brightness).round() as u8;
    }
}
