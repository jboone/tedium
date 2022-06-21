
#[derive(Copy, Clone, Debug)]
pub enum DetectionEvent {

}

pub trait Detector {
    fn advance(&mut self, sample: f32) -> Option<DetectionEvent>;
}

pub mod goertzel;
pub mod dtmf;
