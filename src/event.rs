
use crate::node::Process;
use crate::Node;
use crate::sampling::SampleIdx;
use crate::SamplingRate;

#[derive(Clone)]
pub struct Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    sample: SampleIdx,
    fu: fn(&mut F) -> (),

    s: std::marker::PhantomData<S>,
    f: std::marker::PhantomData<F>,
}

impl<S, F> Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    pub fn new(fu: fn(&mut F) -> (), time: std::time::Duration, sample_rate: SamplingRate) -> Self {
        let idx_sample = sample_rate.from_time(time);
        
        Self {
            sample: idx_sample,
            fu,
            s: std::marker::PhantomData,
            f: std::marker::PhantomData,
        }
    }

    pub fn play_on<const N: usize>(self, node: &mut Node<S, F, N>) {
        (self.fu)(&mut node.f);
    }

    pub fn get_sample_idx(&self) -> SampleIdx {
        self.sample
    }
}

impl<S, F> PartialEq for Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.sample == other.sample
    }
}

impl<S, F> Eq for Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{ }

impl<S, F> PartialOrd for Event<S, F> 
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Order event by decreasing time so that
// nearest occuring events are pushed to the back of the stack
use std::cmp::Ordering;
impl<S, F> Ord for Event<S, F> 
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    fn cmp(&self, other: &Self) -> Ordering {
        other.sample.cmp(&self.sample)
    }
}