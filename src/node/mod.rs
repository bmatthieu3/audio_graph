use std::sync::{Arc, Mutex};
use std::marker::Send;
use std::collections::HashMap;
pub struct Node<P, S, F, const N: usize>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone,
{
    f: F, // process task
    name: &'static str,
    pub params: P,
    on: bool, // process on

    events: Vec< Event< P > >,

    parents: HashMap<&'static str, Arc<Mutex< dyn NodeTrait<S, N> >>>,
}
pub type Nodes<S, const N: usize> = HashMap<&'static str, Arc<Mutex<dyn NodeTrait<S, N>>>>;

use crate::Event;

use crate::sampling::SampleIdx;
impl<P, S, F, const N: usize> Node<P, S, F, N>
where
    P: Params + Clone + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static,
{
    pub fn new(name: &'static str, p: P, f: F) -> Self {
        Self {
            f: f,
            on: true,
            params: p,
            name: name,
            parents: HashMap::new(),
            events: vec![],
        }
    }

    pub fn add_input<P2, F2>(mut self, input: Node<P2, S, F2, N>) -> Self
    where
        P2: Params + Clone + 'static,
        F2: Process<S, P = P2> + Clone + 'static,
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
                let mut threads = vec![];

                for p in self.parents.values_mut() {
                    let p1 = p.clone();

                    threads.push(std::thread::spawn(move || {
                        let mut b = {
                            let b = vec![S::zero_value(); N]
                                .into_boxed_slice();

                            unsafe { Box::from_raw(Box::into_raw(b) as *mut [S; N]) }
                        };
                        // process this with other cores
                        p1.lock().unwrap().stream_into(&mut b, true);
                        b
                    }));
                }

                for thread in threads {
                    // Wait for the thread to finish. Returns a result.
                    let r = thread.join().unwrap();
                    data.push(r);
                }
            } else {
                let mut b = {
                    let b = vec![S::zero_value(); N]
                        .into_boxed_slice();

                    unsafe { Box::from_raw(Box::into_raw(b) as *mut [S; N]) }
                };

                for p in self.parents.values_mut() {
                    p.lock().unwrap().stream_into(&mut b, false);
                    data.push(b.clone());
                }
            }
        }

        for idx_sample in 0..N {
            let mut input = vec![];
            for buf in &data {
                input.push(buf[idx_sample]);
            }

            while !self.events.is_empty() && self.events.last().unwrap().get_sample_idx() <= SampleIdx(idx_sample) {
                let event = self.events.pop().unwrap();
                event.play_on(self);
            }

            buf[idx_sample] = self.f.process_next_value(&self.params, &input);
        }
    }

    // Register the event in the node or its children
    // return true if a node has been found
    pub fn register_event(&mut self, event: Event<P>) {
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

impl<P, S, F, const N: usize> NodeTrait<S, N> for Node<P, S, F, N>
where
    P: Params + Clone + Send + 'static,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone + 'static
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

impl<P, S, F, const N: usize> Iterator for Node<P, S, F, N>
where 
    P: Params + Clone + Send,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P> + Clone
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        let in_values: Option<Vec<_>> = self.parents
            .values_mut()
            .map(|in_iter| in_iter.lock().unwrap().next())
            .collect();

        if let Some(values) = in_values {
            Some(self.f.process_next_value(&self.params, &values[..]))
        } else {
            None
        }
    }
}

pub trait Process<S>: Send
where 
    S: rodio::Sample + Send
{
    type P: Params;
    fn process_next_value(&mut self, params: &Self::P, inputs: &[S]) -> S;
}

pub trait Params: Send {}
impl Params for () {}

pub mod sinewave;
pub use sinewave::{SineWave, SineWaveParams};
pub mod mixer;
pub use mixer::{Mixer};