# audio_graph

A crate exposing an API for building an audio graph and streaming it into a buffer of samples

## Example

```rust
use audio_graph::{Watcher, Audiograph, Node, Event};
use audio_graph::SineWave;
let sw1 = Node::new("sw1", SineWave::new(0.1, 2500.0));
let sampling_rate = 44100.0;
let mut audio = Audiograph::new(sampling_rate, Watcher::on(sw1));
for i in 0..5 {
    // create the event on a node
    let event = Event::update_params(
        |f: &mut SineWave| {
            f.params.freq *= 1.1;
        },
        std::time::Duration::new(i, 0),
        &audio,
    );
    assert!(audio.register_event("sw1", event));
}
let mut buf = Box::new([0.0; 1000]);
audio.stream_into(&mut buf, true);
```

## Try it

### Run the test

```rust
cargo test -- <TESTNAME>
```

Please uncomment the `play_sound` calls to hear the sounds

### Run the doc

```rust
cargo doc  --no-deps --open
```

See the Audiograph module API doc

### Run the bench

```rust
cargo bench
```

parallelism / sequential comparisons

