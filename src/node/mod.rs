use std::sync::{Arc, Mutex};
use std::marker::Send;
use std::collections::HashMap;
pub struct Node<S, F, const N: usize>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    pub f: F, // process task
    name: &'static str,
    on: bool, // process on

    events: Vec< Event< S, F > >,

    parents: HashMap<&'static str, Arc<Mutex< dyn NodeTrait<S, N> >>>,
}
pub type Nodes<S, const N: usize> = HashMap<&'static str, Arc<Mutex<dyn NodeTrait<S, N>>>>;

use crate::Event;

fn vec_to_slice<S, const N: usize>(input: Vec<S>) -> Box<[S; N]> {
    let input = input
        .into_boxed_slice();
    unsafe { Box::from_raw(
        Box::into_raw(input) as *mut [S; N]
    )}
}

use crate::sampling::SampleIdx;
impl<S, F, const N: usize> Node<S, F, N>
where
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone + 'static,
{
    pub fn new(name: &'static str, f: F) -> Self {
        Self {
            f: f,
            on: true,
            name: name,
            parents: HashMap::new(),
            events: vec![],
        }
    }

    pub fn add_input<F2>(mut self, input: Node<S, F2, N>) -> Self
    where
        F2: Process<S> + Clone + 'static,
    {
        self.parents.insert(input.name, Arc::new(Mutex::new(input)));
        self
    }

    fn collect_nodes(&self, nodes: &mut Nodes<S, N>) {
        for (name, parent) in self.parents.iter() {
            nodes.insert(name, parent.clone());

            parent.lock().unwrap().collect_nodes(nodes);
        }
    }

    pub fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        let num_parents = self.parents.len();
        let mut data = Vec::with_capacity(num_parents);

        if num_parents > 0 {
            if multithreading {
                let (tx, rx) = std::sync::mpsc::channel();

                for parent in self.parents.values_mut() {
                    let parent = parent.clone();
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        // Create a buffer on the thread
                        let mut buffer = vec_to_slice(vec![S::zero_value(); N]);

                        // Stream into it
                        parent.lock()
                            .unwrap()
                            .stream_into(&mut buffer, true);

                        // Send the processed data to the calling thread (receiver)
                        tx.send(buffer)
                            .unwrap();
                    });
                }
                drop(tx);

                while let Ok(buffer) = rx.recv() {
                    data.push(buffer);
                }
            } else {
                let mut buffer = vec_to_slice(vec![S::zero_value(); N]);

                for parent in self.parents.values_mut() {
                    parent.lock()
                        .unwrap()
                        .stream_into(&mut buffer, false);

                    data.push(buffer.clone());
                }
            }
        }

        let mut input = Vec::with_capacity(data.len());
        for idx_sample in 0..N {
            for buf in &data {
                input.push(buf[idx_sample]);
            }

            while !self.events.is_empty() && self.events.last().unwrap().get_sample_idx() <= SampleIdx(idx_sample) {
                let event = self.events.pop().unwrap();
                event.play_on(self);
            }

            buf[idx_sample] = self.f.process_next_value(&input);

            input.clear();
        }
    }

    // Register the event in the node or its children
    // return true if a node has been found
    pub fn register_event(&mut self, event: Event<S, F>) {
        // Add the event to the current node
        self.events.push(event);
        // sort by sample idx so that we can only execute the first one
        self.events.sort();
    }
}

// The Node trait responsible for retrieving
use std::any::Any;
pub trait NodeTrait<S, const N: usize>: Iterator<Item=S> + Send
where 
    S: rodio::Sample + Send + 'static
{
    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool);
    fn collect_nodes(&self, nodes: &mut Nodes<S, N>);
    fn as_mut_any(&mut self) -> &mut dyn Any;
}

impl<S, F, const N: usize> NodeTrait<S, N> for Node<S, F, N>
where 
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone
{
    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        self.stream_into(buf, multithreading);
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn collect_nodes(&self, nodes: &mut Nodes<S, N>) {
        self.collect_nodes(nodes);
    }
}

impl<S, F, const N: usize> Iterator for Node<S, F, N>
where 
    S: rodio::Sample + Send + 'static,
    F: Process<S> + Clone
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        let in_values: Option<Vec<_>> = self.parents
            .values_mut()
            .map(|in_iter| in_iter.lock().unwrap().next())
            .collect();

        if let Some(values) = in_values {
            Some(self.f.process_next_value(&values[..]))
        } else {
            None
        }
    }
}

pub trait Process<S>: Send
where 
    S: rodio::Sample + Send
{
    fn process_next_value(&mut self, inputs: &[S]) -> S;
}

pub mod sinewave;
pub use sinewave::SineWave;
pub mod mixer;
pub use mixer::{Mixer};
pub mod multiplier;
pub use multiplier::Multiplier;
