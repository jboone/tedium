use std::f32::consts::TAU;

use super::ToneGenerator;

pub struct DualToneGenerator {
    phase_0: f32,
    phase_advance_0: f32,
    phase_1: f32,
    phase_advance_1: f32,
    amplitude: f32,
    output: f32,
}

impl DualToneGenerator {
    pub fn new(freq_1_hz: f32, freq_2_hz: f32) -> Self {
        Self {
            phase_0: 0.0,
            phase_advance_0: TAU * freq_1_hz / 8000.0,
            phase_1: 0.0,
            phase_advance_1: TAU * freq_2_hz / 8000.0,
            amplitude: 0.1,
            output: 0.0,
        }
    }
}

impl ToneGenerator for DualToneGenerator {
    fn output(&self) -> f32 {
        self.output
    }

    fn advance(&mut self) {
        self.output = (self.phase_0.sin() + self.phase_1.sin()) * 0.5 * self.amplitude;
        self.phase_0 = (self.phase_0 + self.phase_advance_0) % TAU;
        self.phase_1 = (self.phase_1 + self.phase_advance_1) % TAU;
    }
}
