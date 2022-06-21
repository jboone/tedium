use std::ops::RangeInclusive;

use super::goertzel::GoertzelDetector;
use super::DetectionEvent;

struct DetectorGroup {
    detectors: [GoertzelDetector; 4],
    n: usize,
    iteration: usize,
}

impl DetectorGroup {
    fn new(frequencies: [f32; 4], n: usize, initial_iteration: usize) -> Self {
        let detectors = frequencies.map(|frequency| GoertzelDetector::from_hz(frequency, n));

        Self {
            detectors,
            n,
            iteration: initial_iteration,
        }
    }

    fn poll(&mut self) -> [f32; 4] {
        [0, 1, 2, 3].map(|n| self.detectors[n].poll())
    }

    fn iterate(&mut self, x_n: f32) -> Option<[f32; 4]> {
        assert!(self.iteration < self.n);

        for detector in &mut self.detectors {
            detector.iterate(self.iteration, x_n);
        }
        self.iteration += 1;

        if self.iteration == self.n {
            self.iteration = 0;

            Some(self.poll())
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum DetectionState {
    S0,
    S1,
    S2,
    S3,
}

struct DetectionStateMachine {
    state: DetectionState,
    last_detection: Option<char>,
}

impl DetectionStateMachine {
    fn new() -> Self {
        Self {
            state: DetectionState::S0,
            last_detection: None,
        }
    }

    fn feed(&mut self, detection: Option<char>) -> Option<DetectionEvent> {
        // Borrowed a lot of implementation ideas from:
        // https://users.ece.utexas.edu/~bevans/papers/1998/dtmf/dtmf.pdf

        let valid = detection.is_some() && self.last_detection.is_some();
        let same = valid && (detection == self.last_detection);
        let length = true; // Stubbed, as I think the detection code already handles this.
        let pause = detection.is_none() && self.last_detection.is_none();

        use DetectionState::*;

        let (new_state, new_tone) = match self.state {
            S0 => {
                match (length, same, valid) {
                      (  true, true, false) => (S1, false), // length & same & !!valid => S1
                      (     _, true, true ) => (S2, false), // same & valid => S2
                      _ => (self.state, false),
                }
            },
            S1 => {
                match (length, valid,  same, pause) {
                      ( false,     _,     _,     _) |
                      (     _,     _, false,     _) |
                      (     _,     _,     _,  true) => (S0, false), // !length + !same + !pause => S0
                      (     _,  true,  true,     _) => (S3,  true), // valid & same => S3
                      _  => (self.state, false),
                }
            }
            S2 => {
                match (length,  same, pause) {
                      (     _,     _,  true) => (S0, false),    // pause => S0
                      ( false,     _,     _) |
                      (     _, false,     _) => (S3, false),    // !length + !same => S3
                      (  true,  true,     _) => (S3,  true),    // length & same => S3
                      _  => (self.state, false),
                }
            },
            S3 => {
                match pause {
                    true => (S0, false),    // pause => S0
                    _    => (self.state, false),
                }
            },
        };
        self.state = new_state;

        if new_tone {
            eprintln!("{detection:?}");
        }

        self.last_detection = detection;

        None
    }
}

pub struct Detector {
    // Two phases of low detectors to get better time resolution.
    tones_low: [DetectorGroup; 2],
    tones_high: DetectorGroup,
    state_machine: DetectionStateMachine,
}

impl Detector {
    pub fn new() -> Self {
        static FREQUENCIES_LOW:  [f32; 4] = [ 697.0,  770.0,  852.0,  941.0];
        static FREQUENCIES_HIGH: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];

        Self {
            tones_low: [
                DetectorGroup::new(FREQUENCIES_LOW, 212,   0),
                DetectorGroup::new(FREQUENCIES_LOW, 212, 106),
            ],
            tones_high: DetectorGroup::new(FREQUENCIES_HIGH, 106, 0),
            state_machine: DetectionStateMachine::new(),
        }
    }

    fn detect(&self, low_powers: [f32; 4], high_powers: [f32; 4]) -> Option<char> {
        // G.711 says a full-scale (+/-8159) sine wave is +3.17 dBm0.

        const DETECT_POWER_RANGE: RangeInclusive<f32> = -25.0..=0.0;
        const TWIST_RANGE: RangeInclusive<f32> =  -8.0..=4.0;

        let mut low_powers = [0, 1, 2, 3].map(|n| (n, low_powers[n]));
        low_powers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let (power_low_index, power_low_max) = low_powers[0];
        let detect_low = DETECT_POWER_RANGE.contains(&power_low_max);
        if !detect_low {
            return None;
        }
        let powers_low_diff = power_low_max - low_powers[1].1;
        if powers_low_diff < 10.0 {
            return None;
        }

        let mut high_powers = [0, 1, 2, 3].map(|n| (n, high_powers[n]));
        high_powers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let (power_high_index, power_high_max) = high_powers[0];
        let detect_high = DETECT_POWER_RANGE.contains(&power_high_max);
        if !detect_high {
            return None;
        }
        let powers_high_diff = power_high_max - high_powers[1].1;
        if powers_high_diff < 10.0 {
            return None;
        }

        let twist = power_high_max - power_low_max;
        let twist_ok = TWIST_RANGE.contains(&twist);
        if !twist_ok {
            return None;
        }

        static KEY_MAP: [[char; 4]; 4] = [
            ['1', '2', '3', 'A'],
            ['4', '5', '6', 'B'],
            ['7', '8', '9', 'C'],
            ['*', '0', '#', 'D'],
        ];

        let row = power_low_index;
        let column = power_high_index;

        let key = KEY_MAP[row][column];

        // eprintln!("dtmf detect: {low_powers:3.0?} {high_powers:3.0?} twist={twist:4.1} {key}");

        Some(key)
    }
}

impl super::Detector for Detector {
    fn advance(&mut self, x_n: f32) -> Option<DetectionEvent> {

        let mut low_result = None;
        for tones_low in &mut self.tones_low {
            if let Some(result) = tones_low.iterate(x_n) {
                assert!(low_result.is_none());
                low_result = Some(result);
            }
        }

        if let Some(high_result) = self.tones_high.iterate(x_n) {
            // High tones detection phase. We should also have a result
            // from one of the two groups of low detectors.
            if let Some(low_result) = low_result {
                let detection = self.detect(low_result, high_result);
                self.state_machine.feed(detection)
            } else {
                unreachable!();
            }
        } else {
            None
        }
    }
}
