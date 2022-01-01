use crate::node::{Process, Params, Node, NodeTrait};

use std::collections::HashMap;

use crate::node::Nodes;
pub struct Audiograph<'a, P, S, F, const N: usize>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static
{
    watcher: Watcher<'a, P, S, F, N>,
    sample_rate: SamplingRate,
    nodes: Nodes<S, N>,
}

use std::sync::{Mutex, Arc};
use crate::Event;
use crate::SamplingRate;
impl<'a, P, S, F, const N: usize> Audiograph<'a, P, S, F, N>
where 
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static
{
    pub fn new<T: Into<SamplingRate>>(sample_rate: T, watcher: Watcher<'a, P, S, F, N>) -> Self {
        let sample_rate = sample_rate.into();

        let mut nodes = HashMap::new();
        watcher.collect_nodes(&mut nodes);

        Self {
            sample_rate,
            watcher,
            nodes
        }
    }

    // Watch another node
    //
    // When streaming samples, it will produce samples from this watched node
    // It is supposed to watch a node from the same graph => S is preserved
    pub fn set_watcher<'b, P2, F2>(self, watcher: Watcher<'b, P2, S, F2, N>) -> Audiograph<'b, P2, S, F2, N>
    where
        P2: Params + Clone + 'static,
        F2: Process<S, P = P2> + Clone + 'static
    {
        let mut nodes = HashMap::new();
        watcher.collect_nodes(&mut nodes);

        Audiograph {
            sample_rate: self.sample_rate,
            watcher: watcher,
            nodes: nodes,
        }
    }

    // Add an event to the graph that will be played
    // at a given time
    pub fn search_for(&mut self, name: &'static str) -> Option<&mut Arc<Mutex<dyn NodeTrait<S, N>>>> {
        self.nodes.get_mut(name)
    }

    pub fn register_event<P2, F2>(&mut self, name: &'static str, event: Event<P2>) -> bool
    where
        P2: Params + Clone + 'static,
        F2: Process<S, P = P2> + Clone + 'static
    {
        if let Some(node) = self.search_for(name) {
            let mut node = node.lock().unwrap();

            if let Some(node) = node.as_mut_any()
                .downcast_mut::<Node<P2, S, F2, N>>() {
                    node.register_event(event);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    // Stream the next N samples into a buffer of size N allocated on the heap
    pub fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        self.watcher.stream_into(buf, multithreading);
    }

    pub fn get_sampling_rate(&self) -> SamplingRate {
        self.sample_rate
    }
}

pub struct Watcher<'a, P, S, F, const N: usize>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static,
{
    node: &'a mut Node<P, S, F, N>
}


impl<'a, S, P, F, const N: usize> Watcher<'a, P, S, F, N>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static,
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
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static,
{
    type Target = Node<P, S, F, N>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
impl<'a, P, S, F, const N: usize> DerefMut for Watcher<'a, P, S, F, N>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<'a, P, S, F, const N: usize> Iterator for Audiograph<'a, P, S, F, N>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        self.watcher.next()
    }
}