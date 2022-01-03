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
use std::sync::{Arc, Mutex};
impl<S, const N: usize> Audiograph<S, N>
where
    S: rodio::Sample + Send + 'static,
{
    /// Crate a new audio graph
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - The sample rate given as number of samples per second
    /// * `root` - The root node of the graph
    ///
    /// # Examples
    ///
    /// ```
    /// use audio_graph::{Watcher, Audiograph, Node};
    /// use audio_graph::SineWave;
    /// let sw1 = Node::new("sinewave", SineWave::new(0.1, 2500.0));
    /// let w = Watcher::on(sw1);
    /// let mut audio = Audiograph::new(44100.0, w);
    /// let mut buf = Box::new([0.0; 1000]);
    /// audio.stream_into(&mut buf, true);
    /// ```
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

    /// Give a another set of nodes to the graph
    ///
    /// # Arguments
    ///
    /// * `root` - New graph root
    pub fn set_root(&mut self, root: Watcher<S, N>) {
        let mut nodes = HashMap::new();
        root.collect_nodes(&mut nodes);

        self.root = root;
        self.nodes = nodes;
    }

    /// Give a another set of nodes to the graph
    ///
    /// # Arguments
    ///
    /// * `root` - New graph root
    pub fn add_input_to<F2>(&mut self, name: &'static str, input: Node<S, F2, N>) -> bool
    where
        F2: Process<S> + Clone + 'static,
    {
        let input_name = input.name;
        let input = Arc::new(Mutex::new(input));

        // 1. add to the hierarchy
        let node_found = if let Some(node) = self.nodes.get_mut(name) {
            // We found a node
            let mut node = node.lock().unwrap();

            node.add_input_trait_object(input_name, input.clone());

            true
        } else {
            false
        };

        // 2. add to the graph hash map
        if node_found {
            self.nodes.insert(input_name, input);
        }

        node_found
    }

    /// Register an event to a node by its name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the root to register the event too
    /// * `event` - The event to register
    ///
    /// # Examples
    ///
    /// ```
    /// use audio_graph::{Watcher, Audiograph, Node, Event};
    /// use audio_graph::SineWave;
    /// let sw1 = Node::new("sw1", SineWave::new(0.1, 2500.0));
    /// let sampling_rate = 44100.0;
    /// let mut audio = Audiograph::new(sampling_rate, Watcher::on(sw1));
    /// for i in 0..5 {
    ///     // create the event on a node
    ///     let event = Event::update_params(
    ///         |f: &mut SineWave| {
    ///             f.params.freq *= 1.1;
    ///         },
    ///         std::time::Duration::new(i, 0),
    ///         &audio,
    ///     );
    ///     assert!(audio.register_event("sw1", event));
    /// }
    /// let mut buf = Box::new([0.0; 1000]);
    /// audio.stream_into(&mut buf, true);
    /// ```
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

    /// Delete a node by its name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the root to delete
    ///
    /// # Return
    ///
    /// * true whether a node has been found
    pub fn delete_node(&mut self, name: &'static str) -> bool {
        let mut nodes_to_remove = HashSet::new();

        let node_found = self.root.delete_node(name, &mut nodes_to_remove);

        self.nodes.retain(|name, _| !nodes_to_remove.contains(name));

        node_found
    }

    /// Stream the next N samples into a buffer of size N allocated on the heap
    ///
    /// # Arguments
    ///
    /// * `buf` - The buffer to fill
    /// * `multithreading` - Enable multithreading. The streaming of the parent nodes is multithreaded.
    ///   Each parent buffer is filled in a separate thread. Once all the parents buffers are computed,
    ///   we compute the root buffer in the main thread.
    ///
    /// # Example
    ///
    /// ```
    /// use audio_graph::{Watcher, Audiograph, Node, Event};
    /// use audio_graph::SineWave;
    /// let sw1 = Node::new("sw1", SineWave::new(0.1, 2500.0));
    /// let sampling_rate = 44100.0;
    /// let mut audio = Audiograph::new(sampling_rate, Watcher::on(sw1));
    /// for i in 0..5 {
    ///     // create the event on a node
    ///     let event = Event::update_params(
    ///         |f: &mut SineWave| {
    ///             f.params.freq *= 1.1;
    ///         },
    ///         std::time::Duration::new(i, 0),
    ///         &audio,
    ///     );
    ///     assert!(audio.register_event("sw1", event));
    /// }
    /// let mut buf = Box::new([0.0; 1000]);
    /// audio.stream_into(&mut buf, true);
    /// ```
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
