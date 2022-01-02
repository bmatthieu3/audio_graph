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

    // Sample 
    const DURATION_SECS: f32 = 5.0;
    const NUM_SAMPLES: usize = (DURATION_SECS * 44100.0) as usize;

    fn create_empty_buffer<const N: usize>() -> Box<[f32; N]> {
        let buf = vec![0.0; N]
            .into_boxed_slice();
        unsafe {
            Box::from_raw(Box::into_raw(buf) as *mut [f32; N])
        }
    }

    fn play_sound(buf: Box<[f32; NUM_SAMPLES]>) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let source = rodio::buffer::SamplesBuffer::new(1, 44100, buf.to_vec());
        sink.append(source);

        // The sound plays in a separate thread. This call will block the current thread until the sink
        // has finished playing all its queued sounds.
        sink.sleep_until_end();
    }

    #[test]
    fn simple_sinewave_graph() {
        let mut sw1 = Node::new(
            "sinewave",
            SineWave::new(0.1, 2500.0)
        );

        let mut buf = Box::new([0.0; NUM_SAMPLES]);
        let w = Watcher::on(sw1);
        let mut audio = Audiograph::new(44100.0, w);
        audio.stream_into(&mut buf, true);
    }

    #[test]
    fn remove_the_whole_audiograph() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0.1, 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        );
        mixer
        .add_input(sw1)
        .add_input(sw2);

        let w = Watcher::on(mixer);
        let mut audio = Audiograph::new(44100.0, w);

        // remove the root node
        assert!(audio.delete_node("mixer"));

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        // Stream into the buffer
        audio.stream_into(&mut buf, true);

        // Check that the streaming has not changed the buffer since
        // the graph empty
        assert_eq!(buf, create_empty_buffer::<NUM_SAMPLES>());
    }

    #[test]
    fn mixer_audio_graph() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0.1, 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        );
        mixer
        .add_input(sw1)
        .add_input(sw2);

        let w = Watcher::on(mixer);
        let mut audio = Audiograph::new(44100.0, w);

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);
    }

    #[test]
    fn delete_node_from_audio_graph() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );
        let sw2 = Node::new(
            "sw2",
            SineWave::new(0.2, 9534.0)
        );
        let mut mixer = Node::new(
            "mixer",
            Mixer
        );
        mixer
        .add_input(sw1)
        .add_input(sw2);

        let w = Watcher::on(mixer);
        let mut audio = Audiograph::new(44100.0, w);

        let event = Event::update_params(
            |f: &mut SineWave| {
                f.params.freq *= 1.1;
            },
            std::time::Duration::new(2, 0),
            audio.get_sampling_rate()
        );

        assert!(audio.register_event("sw2", event));

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);

        play_sound(buf);
    }

    #[test]
    fn simple_event() {
        let sw1 = Node::new(
            "sw1",
            SineWave::new(0.1, 2500.0)
        );

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, Watcher::on(sw1));

        for i in 0..5 {
            // create the event on a node
            let event = Event::update_params(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw1", event));
        }

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);

        play_sound(buf);
    }

    #[test]
    fn event_on_graph() {
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
        );
        mixer
        .add_input(sw1)
        .add_input(sw2);

        let w = Watcher::on(mixer);

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, w);

        for i in 0..5 {
            // create the event on a node
            let e1 = Event::update_params(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw1", e1));
            let e2 = Event::update_params(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw2", e2));
            let e3 = Event::update_params(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(!audio.register_event("sw3", e3));
        }

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);
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
        );
        mult
        .add_input(lfo)
        .add_input(sw1);

        let w = Watcher::on(mult);

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, w);

        for i in 0..5 {
            // create the event on a node
            let event = Event::update_params(
                |f: &mut SineWave| {
                    f.params.freq *= 1.1;
                },
                std::time::Duration::new(i, 0),
                audio.get_sampling_rate()
            );
            assert!(audio.register_event("sw1", event));
        }

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);

        play_sound(buf);
    }

    #[test]
    fn note_on() {
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
        );
        mult
        .add_input(lfo)
        .add_input(sw1);

        let w = Watcher::on(mult);

        let sampling_rate = 44100.0;
        let mut audio = Audiograph::new(sampling_rate, w);

        // create the event on a node
        let e1 = Event::<_, SineWave, NUM_SAMPLES>::note_off(
            std::time::Duration::new(1, 0),
            audio.get_sampling_rate()
        );
        assert!(audio.register_event("sw1", e1));
        
        let e2 = Event::<_, SineWave, NUM_SAMPLES>::note_on(
            std::time::Duration::new(2, 0),
            audio.get_sampling_rate()
        );
        assert!(audio.register_event("sw1", e2));

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);

        play_sound(buf);
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
        );
        mixer
        .add_input(sw1)
        .add_input(sw2);

        let w = Watcher::on(mixer);
        let mut audio = Audiograph::new(44100.0, w);

        let mut buf = create_empty_buffer::<NUM_SAMPLES>();
        audio.stream_into(&mut buf, true);
    }
}