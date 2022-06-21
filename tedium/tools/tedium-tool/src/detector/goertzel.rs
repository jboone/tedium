use std::f32::consts::TAU;

use num_complex::Complex;

pub struct GoertzelDetector {
    w_n: f32,
    w_n_z1: f32,
    w_n_z2: f32,
    k_fb: f32,
    k_ff: Complex<f32>,
    n: usize,
}

impl GoertzelDetector {
    fn new(m: f32, n: usize) -> Self {
        let tau_m_over_n = TAU * m / (n as f32);
        Self {
            w_n: 0.0,
            w_n_z1: 0.0,
            w_n_z2: 0.0,
            k_fb: tau_m_over_n.cos() * 2.0,
            k_ff: -(Complex::new(0.0, -tau_m_over_n).exp()),
            n,
        }
    }

    pub fn from_hz(frequency_hz: f32, n: usize) -> Self {
        let m = frequency_hz / (8000.0 / n as f32);
        Self::new(m, n)
    }

    pub fn poll(&mut self) -> f32 {
        let y_n = self.w_n + self.w_n_z1 * self.k_ff;
        let magnitude = y_n.norm();

        self.w_n = 0.0;
        self.w_n_z1 = 0.0;
        self.w_n_z2 = 0.0;

        // TODO: This *seems* to be the power of the signal, and dBm0-ish...
        (magnitude / (self.n as f32)).log10() * 20.0
    }

    pub fn iterate(&mut self, iteration: usize, x_n: f32) {
        debug_assert!(iteration < self.n);

        self.w_n_z2 = self.w_n_z1;
        self.w_n_z1 = self.w_n;
        self.w_n = x_n + self.w_n_z1 * self.k_fb - self.w_n_z2;
    }
}
/*
impl GoertzelDetector {
    fn new(m: f32, n: usize) -> Self {
        let n_plus_1 = n + 1;

        Self {
            w_n: 0.0,
            w_n_z1: 0.0,
            w_n_z2: 0.0,
            k_fb: (TAU * m / n as f32).cos() * 2.0,
            n_plus_1,
        }
    }

    pub fn from_hz(frequency_hz: f32, poll_interval_samples: usize) -> Self {
        let n = poll_interval_samples - 1;
        let m = frequency_hz / (8000.0 / n as f32);
        Self::new(m, n)
    }

    pub fn poll(&mut self) -> f32 {
        // Used if you want to avoid complex numbers and get a power output
        // (thereby avoiding a square root operation), but requires an output
        // rate of once every N+1 samples (not every N).
        let power = self.w_n_z1 * self.w_n_z1
                   + self.w_n_z2 * self.w_n_z2
                   - self.w_n_z1 * self.w_n_z2 * self.k_fb;

        self.w_n = 0.0;
        self.w_n_z1 = 0.0;
        self.w_n_z2 = 0.0;

        // TODO: This *seems* to be the power of the signal, and dBm0-ish...
        (power.sqrt() / ((self.n_plus_1 - 1) as f32)).log10() * 10.0
    }

    pub fn iterate(&mut self, iteration: usize, x_n: f32) {
        // We're using the N+1 implementation that produces a power
        // output and eliminates a complex number calculation and
        // square root operation.
        if iteration < self.n_plus_1 {
            self.w_n_z2 = self.w_n_z1;
            self.w_n_z1 = self.w_n;
            self.w_n = x_n + self.w_n_z1 * self.k_fb - self.w_n_z2;
        }
    }
}
*/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_iterations() {
        power(150);
        power(200);
        power(250);
        power(350);
        power(500);
        power(1000);
        power(10000);
    }

    fn power(iterations: usize) {
        let sampling_rate_hz = 8000.0;

        let frequency_hz = 697.0;
        let mut dut = GoertzelDetector::from_hz(697.0, iterations);

        let mut power_sum = 0.0;
        for iteration in 0..iterations {
            let w = (TAU * frequency_hz / sampling_rate_hz) * iteration as f32;
            let sample = w.sin() * 1.414;
            power_sum += sample * sample;
            dut.iterate(iteration, sample);
        }
        let power = power_sum / (iterations as f32);
        let power_db = power.log10() * 10.0;
        let rms = power.sqrt();
        let rms_db = rms.log10() * 20.0;

        // ampl    out    in pow   in rms
        //  0.5  -6.0dB   -9.0dB   -9.0dB
        //  1.0  -3.0dB   -3.0dB   -3.0dB
        //  2.0   0.0dB    3.0dB    3.0dB
        // 10.0   7.0dB   17.0dB   17.0dB

        // TODO: There's a lot unresolved here about power levels in this test, and coming out of the detector.

        println!("iterations: {iterations}, poll output: {}, power from real samples: {}/{}dB or, RMS {}/{}dB", dut.poll(), power, power_db, rms, rms_db);
    }
}
