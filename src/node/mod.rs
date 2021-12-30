use std::sync::{Arc, Mutex};
use std::marker::Send;
use std::collections::HashMap;
pub struct Node<P, S, F, const N: usize>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    f: F, // process task
    name: &'static str,
    params: P,
    on: bool, // process on

    parents: HashMap<&'static str, Arc<Mutex< dyn NodeTrait<S, N> >>>,
}

impl<P, S, F, const N: usize> Node<P, S, F, N>
where
    P: Params,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>,
{
    pub fn new(name: &'static str, p: P, f: F) -> Self {
        Self {
            f: f,
            on: true,
            params: p,
            name: name,
            parents: HashMap::new()
        }
    }

    pub fn add_input<P2, F2>(mut self, input: Node<P2, S, F2, N>) -> Self
    where
        P2: Params + 'static,
        F2: Process<S, P = P2> + 'static,
    {
        self.parents.insert(input.name, Arc::new(Mutex::new(input)));
        self
    }

    /*fn remove_input(&mut self, name: &'a str) {
        self.parents.remove(name);
    }*/

    /*fn params(&mut self) -> &mut Box<dyn Params> {
        &mut self.params
    }*/

    /*fn apply_event<C: FnOnce(&mut Self) -> ()>(&mut self, time: std::time::Duration, f: C) {
        // Apply the closure on the node
        (f)(self)
    }*/
}

// The Node trait responsible for retrieving
pub trait NodeTrait<S, const N: usize>: Iterator<Item=S> + Send
where 
    S: rodio::Sample + Send + 'static
{
    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool);
}

impl<P, S, F, const N: usize> NodeTrait<S, N> for Node<P, S, F, N>
where
    P: Params + Send,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>
{
    fn stream_into(&mut self, buf: &mut Box<[S; N]>, multithreading: bool) {
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

            buf[idx_sample] = self.f.process_next_value(&self.params, &input);
        }
    }
}

impl<P, S, F, const N: usize> Iterator for Node<P, S, F, N>
where 
    P: Params + Send,
    S: rodio::Sample + Send + 'static,
    F: Process<S, P = P>
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

pub trait Process<S>: Sized + Send
where 
    S: rodio::Sample + Send
{
    type P: Params;
    fn process_next_value(&mut self, params: &Self::P, inputs: &[S]) -> S;
}

pub trait Params: Sized + Send {
    fn apply_event<F: FnOnce(&mut Self) -> ()>(&mut self, f: F) {
        (f)(self)
    }
}
impl Params for () {}

pub mod sinewave;
pub use sinewave::{SineWave, SineWaveParams};
pub mod mixer;
pub use mixer::{Mixer};