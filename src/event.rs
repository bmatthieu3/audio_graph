
use crate::node::Process;
use crate::Node;
use crate::sampling::SampleIdx;
use crate::SamplingRate;

#[derive(Clone)]
pub enum Event<S, F, const N: usize>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    UpdateParams {
        sample: SampleIdx,
        fu: fn(&mut F, &mut Audiograph<S, F, N>) -> (),

        s: std::marker::PhantomData<S>,
        f: std::marker::PhantomData<F>,
    },
    NoteOff {
        sample: SampleIdx,
    },
    NoteOn {
        sample: SampleIdx,
    }
}

impl<S, F> Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    pub fn update_params(fu: fn(&mut F, &mut Audiograph<S, F, N>) -> (), time: std::time::Duration, sample_rate: SamplingRate) -> Self {
        let idx_sample = sample_rate.from_time(time);
        
        Event::UpdateParams {
            sample: idx_sample,
            fu,
            s: std::marker::PhantomData,
            f: std::marker::PhantomData,
        }
    }

    pub fn note_on(time: std::time::Duration, sample_rate: SamplingRate) -> Self {
        let idx_sample = sample_rate.from_time(time);
        
        Event::NoteOn {
            sample: idx_sample,
        }
    }

    pub fn note_off(time: std::time::Duration, sample_rate: SamplingRate) -> Self {
        let idx_sample = sample_rate.from_time(time);
        
        Event::NoteOff {
            sample: idx_sample,
        }
    }

    pub fn play_on<const N: usize>(self, node: &mut Node<S, F, N>) {
        match self {
            Event::UpdateParams {
                fu,
                ..
            } => (fu)(&mut node.f),
            Event::NoteOn { .. } => node.on = true,
            Event::NoteOff { .. } => node.on = false,
        }
    }

    pub fn get_sample_idx(&self) -> SampleIdx {
        match self {
            Event::UpdateParams {
                sample,
                ..
            } => {
                *sample
            },
            Event::NoteOff { sample } => *sample,
            Event::NoteOn { sample } => *sample
        }
    }
}

impl<S, F> PartialEq for Event<S, F>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.get_sample_idx() == other.get_sample_idx()
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
        other.get_sample_idx().cmp(&self.get_sample_idx())
    }
}