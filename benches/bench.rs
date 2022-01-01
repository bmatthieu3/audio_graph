use criterion::{criterion_group, criterion_main, Criterion};
use audio_graph::{Node, Watcher, Audiograph};
use audio_graph::node::Process;

use audio_graph::node::{SineWave, Mixer};

fn criterion_benchmark(c: &mut Criterion) {
    let sw1 = Node::new(
        "sw1",
        SineWave::new(0.1, 2500.0)
    );
    let sw2 = Node::new(
        "sw2",
        SineWave::new(0.02, 9534.0)
    );
    let sw3 = Node::new(
        "sw3",
        SineWave::new(0.01, 15534.0)
    );
    let mut mixer = Node::new(
        "mixer",
        Mixer
    )
    .add_input(sw1)
    .add_input(sw2)
    .add_input(sw3);

    const DURATION_IN_SECS: f32 = 100.0;
    const NUM_SAMPLES: usize = (44100.0 * DURATION_IN_SECS) as usize;

    let buf = vec![0.0; NUM_SAMPLES]
        .into_boxed_slice();
    let mut buf = unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; NUM_SAMPLES]) };
    let w = Watcher::on(&mut mixer);
    let mut audio = Audiograph::new(44100.0, w);

    c.bench_function("mixer_parallelism", |b| b.iter(|| audio.stream_into(&mut buf, true)));
    c.bench_function("mixer_sequential", |b| b.iter(|| audio.stream_into(&mut buf, false)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);