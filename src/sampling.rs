#[derive(Clone, Copy)]
#[derive(PartialEq, Eq, Ord, PartialOrd)]
pub struct SampleIdx(pub usize);

#[derive(Clone, Copy)]
pub struct SamplingRate(f32);
impl SamplingRate {
    pub fn from_time(&self, dur: std::time::Duration) -> SampleIdx {
        SampleIdx((self.0 * dur.as_secs_f32()) as usize)
    }
}

impl From<f32> for SamplingRate {
    fn from(a: f32) -> Self {
        SamplingRate(a)
    }
}