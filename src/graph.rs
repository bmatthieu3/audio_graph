use crate::node::{Process, Node, NodeTrait};

use std::collections::HashMap;

use crate::node::Nodes;
pub struct Audiograph<'a, S, F, const N: usize>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static
{
    watcher: Watcher<'a, S, F, N>,
    sample_rate: SamplingRate,
    nodes: Nodes<S, N>,
}
use std::collections::HashSet;
use std::sync::{Mutex, Arc};
use crate::Event;
use crate::SamplingRate;
impl<'a, S, F, const N: usize> Audiograph<'a, S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static
{
    pub fn new<T: Into<SamplingRate>>(sample_rate: T, watcher: Watcher<'a, S, F, N>) -> Self {
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
    pub fn set_watcher<'b, F2>(self, watcher: Watcher<'b, S, F2, N>) -> Audiograph<'b, S, F2, N>
    where
        F2: Process<S> + Clone + 'static
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

    pub fn register_event<F2>(&mut self, name: &'static str, event: Event<S, F2>) -> bool
    where
        F2: Process<S> + Clone + 'static
    {
        if let Some(node) = self.search_for(name) {
            let mut node = node.lock().unwrap();

            if let Some(node) = node.as_mut_any()
                .downcast_mut::<Node<S, F2, N>>() {

                node.register_event(event);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn delete_node(&mut self, name: &'static str) -> bool {
        let mut nodes_to_remove = HashSet::new();
        let node_found = self.watcher.delete_node(name, &mut nodes_to_remove);

        self.nodes.retain(|name, parent| {
            !nodes_to_remove.contains(name)
        });

        node_found
    }

    // Stream the next N samples into a buffer of size N allocated on the heap
    pub fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        self.watcher.stream_into(buf, multithreading);
    }

    pub fn get_sampling_rate(&self) -> SamplingRate {
        self.sample_rate
    }
}

pub struct Watcher<'a, S, F, const N: usize>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    node: &'a mut Node<S, F, N>
}


impl<'a, S, F, const N: usize> Watcher<'a, S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    pub fn on(node: &'a mut Node<S, F, N>) -> Self {
        Self {
            node
        }
    }
}

use std::ops::{DerefMut, Deref};
impl<'a, S, F, const N: usize> Deref for Watcher<'a, S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    type Target = Node<S, F, N>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
impl<'a, S, F, const N: usize> DerefMut for Watcher<'a, S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<'a, S, F, const N: usize> Iterator for Audiograph<'a, S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        self.watcher.next()
    }
}