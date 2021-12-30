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

pub mod graph;
pub use graph::Audiograph;
pub use graph::Watcher;

#[cfg(test)]
mod tests {
    use rodio::{OutputStream, Sink};
    use crate::{Audiograph, Watcher, Node};

    #[test]
    fn simple_sinewave_graph() {
        
        let mut sw1 = Node::new(
            "sinewave",
            crate::node::SineWaveParams { ampl: 0.1, freq: 2500.0 },
            crate::node::SineWave::new()
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
            crate::node::SineWaveParams { ampl: 0.1, freq: 2500.0 },
            crate::node::SineWave::new()
        );
        let sw2 = Node::new(
            "sw2",
            crate::node::SineWaveParams { ampl: 0.02, freq: 9534.0 },
            crate::node::SineWave::new()
        );
        let mut mixer = Node::new(
            "mixer",
            (),
            crate::node::Mixer
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
    fn multithreading() {        
        let sw1 = Node::new(
            "sw1",
            crate::node::SineWaveParams { ampl: 0.1, freq: 2500.0 },
            crate::node::SineWave::new()
        );
        let sw2 = Node::new(
            "sw2",
            crate::node::SineWaveParams { ampl: 0.02, freq: 9534.0 },
            crate::node::SineWave::new()
        );
        let mut mixer = Node::new(
            "mixer",
            (),
            crate::node::Mixer
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