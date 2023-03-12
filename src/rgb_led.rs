pub use rgb::{RGB8, RGBA8};

pub trait RGBABrightnessExt {
    fn get_updated_channels(&mut self) -> RGB8;
}

impl RGBABrightnessExt for RGBA8 {
    /// When called, returns the "real" RGB values, which are used to control the LED
    /// These are the "real" values, since they are scaled by the brightness/ alpha value
    fn get_updated_channels(&mut self) -> RGB8 {
        let rel_brightness: f64 = self.a as f64 / 255.0;
        let r = ((self.r as f64) * rel_brightness).round() as u8;
        let g = ((self.g as f64) * rel_brightness).round() as u8;
        let b = ((self.b as f64) * rel_brightness).round() as u8;

        RGB8::new(r, g, b)
    }
}
