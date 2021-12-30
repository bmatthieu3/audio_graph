use crate::node::{Process, Params, Node, NodeTrait};

pub struct SamplingRate(f32);
impl SamplingRate {
    fn from_time(&self, dur: std::time::Duration) -> IdxSample {
        IdxSample((self.0 * dur.as_secs_f32()) as usize)
    }
}

impl From<f32> for SamplingRate {
    fn from(a: f32) -> Self {
        SamplingRate(a)
    }
}

struct IdxSample(usize);

pub struct Audiograph<'a, P, S, F, const N: usize>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>
{
    watcher: Watcher<'a, P, S, F, N>,
    sample_rate: SamplingRate,
}

impl<'a, P, S, F, const N: usize> Audiograph<'a, P, S, F, N>
where 
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>
{
    pub fn new<T: Into<SamplingRate>>(sample_rate: T, watcher: Watcher<'a, P, S, F, N>) -> Self {
        let sample_rate = sample_rate.into();
        Self {
            sample_rate,
            watcher
        }
    }

    // Watch another node
    //
    // When streaming samples, it will produce samples from this watched node
    // It is supposed to watch a node from the same graph => S is preserved
    pub fn set_watcher<'b, P2, F2, const N2: usize>(self, watcher: Watcher<'b, P2, S, F2, N2>) -> Audiograph<'b, P2, S, F2, N2>
    where
        P2: Params,
        F2: Process<S, P = P2> 
    {
        Audiograph {
            sample_rate: self.sample_rate,
            watcher: watcher
        }
    }

    // Stream the next N samples into a buffer of size N allocated on the heap
    pub fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        self.watcher.stream_into(buf, multithreading);
    }
}

pub struct Watcher<'a, P, S, F, const N: usize>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    node: &'a mut Node<P, S, F, N>
}


impl<'a, S, P, F, const N: usize> Watcher<'a, P, S, F, N>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    pub fn on(node: &'a mut Node<P, S, F, N>) -> Self {
        Self {
            node
        }
    }
}

use std::ops::{DerefMut, Deref};
impl<'a, P, S, F, const N: usize> Deref for Watcher<'a, P, S, F, N>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    type Target = Node<P, S, F, N>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
impl<'a, P, S, F, const N: usize> DerefMut for Watcher<'a, P, S, F, N>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<'a, P, S, F, const N: usize> Iterator for Audiograph<'a, P, S, F, N>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        self.watcher.next()
    }
}