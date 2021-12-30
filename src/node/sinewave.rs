pub struct SineWave {
    step: usize,
}

pub struct SineWaveParams {
    pub ampl: f32,
    pub freq: f32,
}

use super::Params;
impl Params for SineWaveParams {}

impl SineWave {
    pub fn new() -> Self {
        let step = 0;
        Self {
            step,
        }
    }
}

use super::Process;
impl Process<f32> for SineWave {
    type P = SineWaveParams;

    fn process_next_value(&mut self, params: &Self::P, _: &[f32]) -> f32 {
        self.step += 1;
        ((self.step as f32) / 44100.0 * params.freq).sin() * params.ampl
    }
}