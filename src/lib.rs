//! Audio graph processing
//!
//! Provides an implementation of an audio DAG (Directed Acyclic Graph)
//! Features:
//! - Provide traits for the user to implement its own nodes (through the trait Process, and Params)
//! - Use of generics to be compatible with rodio Sample trait
//! - parallel streaming into a buffer of size N

/* --------------------------------------------------------- */
pub mod node;
pub use node::Node;

pub mod sampling;
pub use sampling::{SamplingRate};

pub mod graph;
pub use graph::Audiograph;
pub use graph::Watcher;

pub mod event;
pub use event::Event;

#[cfg(test)]
mod tests {
    use rodio::{OutputStream, Sink};
    use crate::{Audiograph, Watcher, Node, Event};
    use crate::node::*;
    #[test]
    fn simple_sinewave_graph() {
        
        let mut sw1 = Node::new(
            "sinewave",
            SineWave::new(0.1, 2500.0)
        );

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        const DURATION_SECS: f32 = 5.0;
        const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

        let mut buf = Box::new([0.0; NUM_SAMPLES]);
        let w = Watcher::on(&mut sw1);
        let mut audio = Audiograph::new(44100.0, w);
        audio.stream_into(&mut buf, true);

        play_sound(&sink, buf.to_vec());
    }

    fn play_sound(sink: &Sink, buf: Vec<f32>) {
        let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf);
        sink.append(source);

        // The sound plays in a separate thread. This call will block the current thread until the sink
        // has finished playing all its queued sounds.
        sink.sleep_until_end();
    }

    #[test]
    fn mixer_audio_graph() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0., 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        )
        .add_input(sw1)
        .add_input(sw2);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        const DURATION_SECS: f32 = 5.0;
        const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

        let buf = vec![0.0; NUM_SAMPLES]
            .into_boxed_slice();
        let mut buf = unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; NUM_SAMPLES]) };
        let w = Watcher::on(&mut mixer);
        let mut audio = Audiograph::new(44100.0, w);
        audio.stream_into(&mut buf, true);

        play_sound(&sink, buf.to_vec());
    }

    #[test]
    fn event() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0., 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        )
        .add_input(sw1)
        .add_input(sw2);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        const DURATION_SECS: f32 = 5.0;
        const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

        let buf = vec![0.0; NUM_SAMPLES]
            .into_boxed_slice();
        let mut buf = unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; NUM_SAMPLES]) };
        let w = Watcher::on(&mut mixer);

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, w);

        for i in 0..5 {
            // create the event on a node
            let event = Event::new(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw1", event.clone()));
            assert!(audio.register_event("sw2", event.clone()));
            assert!(!audio.register_event("sw3", event));
        }

        audio.stream_into(&mut buf, true);

        play_sound(&sink, buf.to_vec());
    }

    #[test]
    fn lfo_modulating_amplitude() {
        let lfo = Node::new(
            "lfo",
            SineWave::new(1.0, 10.0)
        );
        let sw1 = Node::new(
            "sw1",
            SineWave::new(1.0, 1200.0)
        );
        let mut mult = Node::new(
            "multiplier",
            Multiplier
        )
        .add_input(lfo)
        .add_input(sw1);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        const DURATION_SECS: f32 = 5.0;
        const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

        let buf = vec![0.0; NUM_SAMPLES]
            .into_boxed_slice();
        let mut buf = unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; NUM_SAMPLES]) };
        let w = Watcher::on(&mut mult);

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, w);

        for i in 0..5 {
            // create the event on a node
            let event = Event::new(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw1", event.clone()));
        }

        audio.stream_into(&mut buf, true);

        play_sound(&sink, buf.to_vec());
    }

    #[test]
    fn multithreading() {        
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0.02, 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        )
        .add_input(sw1)
        .add_input(sw2);

        const DURATION_SECS: f32 = 5.0;
        const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

        let buf = vec![0.0; NUM_SAMPLES]
            .into_boxed_slice();
        let mut buf = unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; NUM_SAMPLES]) };
        let w = Watcher::on(&mut mixer);
        let mut audio = Audiograph::new(44100.0, w);
        audio.stream_into(&mut buf, true);
    }
}