#[derive(Clone)]
pub struct SineWave {
    pub params: SineWaveParams,
    step: usize,
}

#[derive(Clone)]
pub struct SineWaveParams {
    pub ampl: f32,
    pub freq: f32,
}

impl SineWave {
    pub fn new(ampl: f32, freq: f32) -> Self {
        let params = SineWaveParams { ampl, freq };
        let step = 0;
        Self { params, step }
    }
}

use super::Process;
impl Process<f32> for SineWave {
    fn process_next_value(&mut self, _: &[f32]) -> f32 {
        self.step += 1;
        ((self.step as f32) / 44100.0 * self.params.freq).sin() * self.params.ampl
    }
}
