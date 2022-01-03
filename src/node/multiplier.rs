/* Mixer */
#[derive(Clone)]
pub struct Multiplier;
use super::Process;
impl Process<f32> for Multiplier {
    fn process_next_value(&mut self, inputs: &[f32]) -> f32 {
        inputs.iter().fold(1.0, |sum, x| sum * x)
    }
}
