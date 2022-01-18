use std::collections::HashMap;
use std::marker::Send;
use std::sync::{Arc, Mutex};
const MAX_NODE_INPUTS: usize = 8;

use rtrb::RingBuffer;

pub struct Node<S, F, const N: usize>
where
    S: rodio::Sample + Send + Sync + 'static,
    F: Process<S> + Clone + 'static,
{
    buf: [S; N],
    pub name: &'static str,
    pub f: F,     // Process
    pub on: bool, // process on

    events: Vec<Event<S, F, N>>,

    parents: HashMap<&'static str, Arc<Mutex<dyn NodeTrait<S, N>>>>,
}
pub(crate) type Nodes<S, const N: usize> = HashMap<&'static str, Arc<Mutex<dyn NodeTrait<S, N>>>>;

use crate::Event;

// Utilitary method to convert an allocated array on the heap
// to a sized boxed slice
unsafe fn vec_to_boxed_slice<T, const N: usize>(input: Vec<T>) -> Box<[T; N]> {
    let input = input.into_boxed_slice();
    Box::from_raw(Box::into_raw(input) as *mut [T; N])
}

use crate::sampling::SampleIdx;
impl<S, F, const N: usize> Node<S, F, N>
where
    S: rodio::Sample + Send + Sync + 'static,
    F: Process<S> + Clone + 'static,
{
    pub fn new(name: &'static str, f: F) -> Self {
        Self {
            buf: [S::zero_value(); N],
            f: f,
            on: true,
            name: name,
            parents: HashMap::new(),
            events: vec![],
        }
    }

    pub fn add_input<F2>(&mut self, input: Node<S, F2, N>) -> &mut Self
    where
        F2: Process<S> + Clone + 'static,
    {
        self.parents.insert(input.name, Arc::new(Mutex::new(input)));
        self
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    // Register the event in the node or its children
    // return true if a node has been found
    pub fn register_event(&mut self, event: Event<S, F, N>) {
        // Add the event to the current node
        self.events.push(event);
        // sort by sample idx so that we can only execute the first one(s)
        self.events.sort();
    }
}

use std::collections::HashSet;
// The Node trait responsible for retrieving
use std::any::Any;
pub trait NodeTrait<S, const N: usize>: Iterator<Item = S> + Send
where
    S: rodio::Sample + Send + Sync + 'static,
{
    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool);
    fn stream_into_rtrb(&mut self, multithreading: bool);

    fn collect_nodes(&self, nodes: &mut Nodes<S, N>);

    fn delete_node(
        &mut self,
        name: &'static str,
        nodes_to_remove: &mut HashSet<&'static str>,
    ) -> bool;
    fn delete_parents_hierarchy(&mut self, nodes_to_remove: &mut HashSet<&'static str>);

    fn add_input_trait_object(
        &mut self,
        name: &'static str,
        input: Arc<Mutex<dyn NodeTrait<S, N>>>,
    );
    fn get_name(&self) -> &'static str;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn get_buf(&self) -> &[S; N];
}

use std::cell::UnsafeCell;

#[derive(Copy, Clone)]
pub struct UnsafeSlice<'a, T> {
    slice: &'a [UnsafeCell<T>],
}
unsafe impl<'a, T: Send + Sync> Send for UnsafeSlice<'a, T> {}
unsafe impl<'a, T: Send + Sync> Sync for UnsafeSlice<'a, T> {}

impl<'a, T> UnsafeSlice<'a, T> {
    pub fn new(slice: &'a mut [T]) -> Self {
        let ptr = slice as *mut [T] as *const [UnsafeCell<T>];
        Self {
            slice: unsafe { &*ptr },
        }
    }
    
    /// SAFETY: It is UB if two threads write to the same index without
    /// synchronization.
    pub unsafe fn write(&self, i: usize, value: T) {
        let ptr = self.slice[i].get();
        *ptr = value;
    }
}

impl<S, F, const N: usize> NodeTrait<S, N> for Node<S, F, N>
where
    S: rodio::Sample + Send + Sync + 'static,
    F: Process<S> + Clone,
{
    fn get_buf(&self) -> &[S; N] {
        &self.buf
    }

    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
        let num_parents = self.parents.len();
        let mut data = Vec::with_capacity(num_parents);

        // 1. run the parents nodes first
        // todo! Handle events that adds a node at runtime!
        if num_parents > 0 {
            if multithreading {
                let (tx, rx) = std::sync::mpsc::channel();

                for parent in self.parents.values_mut() {
                    let parent = parent.clone();
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        // Create a buffer on the thread
                        let mut buffer = unsafe { vec_to_boxed_slice(vec![S::zero_value(); N]) };

                        // Stream into it
                        parent.lock().unwrap().stream_into(&mut buffer, true);

                        // Send the processed data to the calling thread (receiver)
                        tx.send(buffer).unwrap();
                    });
                }
                drop(tx);

                while let Ok(buffer) = rx.recv() {
                    data.push(buffer);
                }
            } else {
                let mut buffer = unsafe { vec_to_boxed_slice(vec![S::zero_value(); N]) };

                for parent in self.parents.values_mut() {
                    parent.lock().unwrap().stream_into(&mut buffer, false);

                    data.push(buffer.clone());
                }
            }
        }

        let mut input = Vec::with_capacity(data.len());
        for idx_sample in 0..N {
            for buf in &data {
                input.push(buf[idx_sample]);
            }

            // As events is sorted by decreasing sample indices, we can only check the last event to be played
            while !self.events.is_empty()
                && self.events.last().unwrap().get_sample_idx() <= SampleIdx(idx_sample)
            {
                let event = self.events.pop().unwrap();
                event.play_on(self);
            }

            buf[idx_sample] = if self.on {
                self.f.process_next_value(&input)
            } else {
                S::zero_value()
            };

            input.clear();
        }
    }

    fn stream_into_rtrb(
        &mut self,
        multithreading: bool,
        //pool: &rayon::ThreadPool
    ) {
        let num_inputs = self.parents.len();
        let mut data = unsafe { vec_to_boxed_slice::<_, MAX_NODE_INPUTS>(
            vec![
                [S::zero_value(); N]; MAX_NODE_INPUTS
            ])
        };
        // 1. run the parents nodes first
        // todo! Handle events that adds a node at runtime!
        if num_inputs > 0 {
            if multithreading {
                //let mut consumers = vec![];
                let mut data_slice = UnsafeSlice::new(&mut data[..]);

                rayon::scope(|s| {
                    for parent in self.parents.values_mut() {
                        let parent = parent.clone();

                        //consumers.push(c);
                        s.spawn(move |_| {
                            let mut input = parent.lock().unwrap();
                            // Stream into it
                            input.stream_into_rtrb(true);
                            let idx = rayon::current_thread_index().unwrap();
                            // Send the processed data to the calling thread (receiver)
                            unsafe { data_slice.write(idx, *input.get_buf()); }
                        });
                    }
                });
            } else {
                let mut i = 0;
                for parent in self.parents.values_mut() {
                    if let Ok(mut parent) = parent.lock() {
                        parent.stream_into_rtrb(false);
                        data[i] = *parent.get_buf();
                        i += 1;
                    }
                }
            }
        }

        let mut input = [S::zero_value(); MAX_NODE_INPUTS];
        for idx_sample in 0..N {
            for idx_input in 0..num_inputs {
                input[idx_input] = data[idx_input][idx_sample];
            }

            // As events is sorted by decreasing sample indices, we can only check the last event to be played
            /*while !self.events.is_empty()
                && self.events.last().unwrap().get_sample_idx() <= SampleIdx(idx_sample)
            {
                let event = self.events.pop().unwrap();
                event.play_on(self);
            }*/

            self.buf[idx_sample] = if self.on {
                self.f.process_next_value(&input[..num_inputs])
            } else {
                S::zero_value()
            };
        }
    }

    fn collect_nodes(&self, nodes: &mut Nodes<S, N>) {
        for (name, parent) in self.parents.iter() {
            nodes.insert(name, parent.clone());

            parent.lock().unwrap().collect_nodes(nodes);
        }
    }

    fn delete_node(
        &mut self,
        name: &'static str,
        nodes_to_remove: &mut HashSet<&'static str>,
    ) -> bool {
        if let Some(node) = self.parents.get(name) {
            // Node found, we first remove all of its parents (by registering them in a set)
            node.lock()
                .unwrap()
                .delete_parents_hierarchy(nodes_to_remove);

            // Then we remove the node found
            self.parents.remove(name);
            // And tag it in the set as well
            nodes_to_remove.insert(name);

            true
        } else {
            // If not found, we loop over the parent hierarchy
            for parent in self.parents.values_mut() {
                if parent.lock().unwrap().delete_node(name, nodes_to_remove) {
                    return true;
                }
            }

            false
        }
    }

    fn delete_parents_hierarchy(&mut self, nodes_to_remove: &mut HashSet<&'static str>) {
        self.parents.retain(|name, parent| {
            // Delete recursively the parents of the parent node
            parent
                .lock()
                .unwrap()
                .delete_parents_hierarchy(nodes_to_remove);

            // Then tag the parent to be removed
            nodes_to_remove.insert(name);

            // Then remove it in the hierarchy
            false
        });
    }

    fn add_input_trait_object(
        &mut self,
        name: &'static str,
        input: Arc<Mutex<dyn NodeTrait<S, N>>>,
    ) {
        self.parents.insert(name, input);
    }

    fn get_name(&self) -> &'static str {
        &self.name
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<S, F, const N: usize> Iterator for Node<S, F, N>
where
    S: rodio::Sample + Send + Sync + 'static,
    F: Process<S> + Clone,
{
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        let in_values: Option<Vec<_>> = self
            .parents
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
    S: rodio::Sample + Send,
{
    fn process_next_value(&mut self, inputs: &[S]) -> S;
}

pub mod sinewave;
pub use sinewave::SineWave;
pub mod mixer;
pub use mixer::Mixer;
pub mod multiplier;
pub use multiplier::Multiplier;
