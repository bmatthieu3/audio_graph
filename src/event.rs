
use crate::node::{Process, Params};
use crate::Node;
use crate::sampling::SampleIdx;
use crate::SamplingRate;

#[derive(Clone)]
pub struct Event<P>
where
    P: Params + Clone + 'static,
{
    sample: SampleIdx,
    fu: fn(&mut P) -> (),

    p: std::marker::PhantomData<P>,
}

impl<P> Event<P>
where
    P: Params + Clone + 'static
{
    pub fn new(fu: fn(&mut P) -> (), time: std::time::Duration, sample_rate: SamplingRate) -> Self {
        let idx_sample = sample_rate.from_time(time);
        
        Self {
            sample: idx_sample,
            fu,
            p: std::marker::PhantomData,
        }
    }

    pub fn play_on<S, F, const N: usize>(self, node: &mut Node<P, S, F, N>)
    where
        P: Params + Clone + 'static,
        S: rodio::Sample + Send + 'static,
        F: Process<S, P = P> + Clone,
    {
        (self.fu)(&mut node.params);
    }

    pub fn get_sample_idx(&self) -> SampleIdx {
        self.sample
    }
}

impl<P> PartialEq for Event<P> 
where
    P: Params + Clone + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.sample == other.sample
    }
}

impl<P> Eq for Event<P> 
where
    P: Params + Clone + 'static,
{ }

impl<P> PartialOrd for Event<P> 
where
    P: Params + Clone + 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Order event by decreasing time so that
// nearest occuring events are pushed to the back of the stack
use std::cmp::Ordering;
impl<P> Ord for Event<P> 
where
    P: Params + Clone + 'static,
{
    fn cmp(&self, other: &Self) -> Ordering {
        other.sample.cmp(&self.sample)
    }
}