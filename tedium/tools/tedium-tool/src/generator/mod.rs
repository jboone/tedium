
pub trait ToneGenerator {
    fn output(&self) -> f32;
    fn advance(&mut self);
}

pub mod dual_tone;
