/* Mixer */
#[derive(Clone)]
pub struct Mixer;
use super::Process;
impl Process<f32> for Mixer {
    fn process_next_value(&mut self, inputs: &[f32]) -> f32 {
        inputs.iter().sum::<f32>()
    }
}
