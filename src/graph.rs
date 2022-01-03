use crate::node::{Node, NodeTrait, Process};

use std::collections::HashMap;

use crate::node::Nodes;
pub struct Audiograph<S, const N: usize>
where
    S: rodio::Sample + Send + 'static,
{
    root: Watcher<S, N>,
    sample_rate: SamplingRate,
    nodes: Nodes<S, N>,
}

use crate::sampling::SamplingRate;
use crate::Event;
use std::collections::HashSet;
impl<S, const N: usize> Audiograph<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    pub fn new<T: Into<SamplingRate>>(sample_rate: T, root: Watcher<S, N>) -> Self {
        let sample_rate = sample_rate.into();

        let mut nodes = HashMap::new();
        root.collect_nodes(&mut nodes);

        Self {
            sample_rate,
            root,
            nodes,
        }
    }

    // Watch another node
    //
    // When streaming samples, it will produce samples from this watched node
    // It is supposed to watch a node from the same graph => S is preserved
    pub fn set_root(&mut self, root: Watcher<S, N>) {
        let mut nodes = HashMap::new();
        root.collect_nodes(&mut nodes);

        self.root = root;
        self.nodes = nodes;
    }

    pub fn add_input_to<F>(&mut self, name: &'static str, input: Node<S, F, N>) -> bool
    where
        F: Process<S> + Clone + 'static,
    {
        if let Some(node) = self.nodes.get_mut(name) {
            // We found a node
            let mut node = node.lock().unwrap();

            if let Some(node) = node.as_mut_any().downcast_mut::<Node<S, F, N>>() {
                node.add_input(input);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn register_event<F>(&mut self, name: &'static str, event: Event<S, F, N>) -> bool
    where
        F: Process<S> + Clone + 'static,
    {
        if let Some(node) = self.nodes.get_mut(name) {
            // We found a node
            let mut node = node.lock().unwrap();

            if let Some(node) = node.as_mut_any().downcast_mut::<Node<S, F, N>>() {
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

        let node_found = self.root.delete_node(name, &mut nodes_to_remove);

        self.nodes.retain(|name, _| !nodes_to_remove.contains(name));

        node_found
    }

    // Stream the next N samples into a buffer of size N allocated on the heap
    pub fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        self.root.stream_into(buf, multithreading);
    }

    pub(crate) fn get_sampling_rate(&self) -> SamplingRate {
        self.sample_rate
    }
}

#[derive(Clone)]
pub struct Sentinel;
impl<S> Process<S> for Sentinel
where
    S: rodio::Sample + Send + 'static,
{
    fn process_next_value(&mut self, inputs: &[S]) -> S {
        if let Some(s) = inputs.first() {
            *s
        } else {
            // The graph is empty => no sound for every sample
            S::zero_value()
        }
    }
}

pub struct Watcher<S, const N: usize>
where
    S: rodio::Sample + Send + 'static,
{
    root: Node<S, Sentinel, N>,
}

impl<S, const N: usize> Watcher<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    pub fn on<F>(node: Node<S, F, N>) -> Self
    where
        F: Process<S> + Clone + 'static,
    {
        let mut sentinel = Node::new("root", Sentinel);
        sentinel.add_input(node);

        Self { root: sentinel }
    }
}

use std::ops::{Deref, DerefMut};
impl<S, const N: usize> Deref for Watcher<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    type Target = Node<S, Sentinel, N>;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}
impl<S, const N: usize> DerefMut for Watcher<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root
    }
}

impl<S, const N: usize> Iterator for Audiograph<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        self.root.next()
    }
}
