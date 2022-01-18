use audio_graph::{Audiograph, Node, Watcher};
use criterion::{criterion_group, criterion_main, Criterion};

use audio_graph::{Mixer, SineWave};

const NUM_SAMPLES: usize = 64;

fn create_empty_buffer<const N: usize>() -> Box<[f32; N]> {
    let buf = vec![0.0; N].into_boxed_slice();
    unsafe { Box::from_raw(Box::into_raw(buf) as *mut [f32; N]) }
}
fn criterion_benchmark(c: &mut Criterion) {
    let sw1 = Node::new("sw1", SineWave::new(0.1, 2500.0));
    let sw2 = Node::new("sw2", SineWave::new(0.02, 9534.0));
    let sw3 = Node::new("sw3", SineWave::new(0.01, 15534.0));
    let mut mixer = Node::new("mixer", Mixer);
    mixer.add_input(sw1).add_input(sw2).add_input(sw3);

    let mut buf = create_empty_buffer::<NUM_SAMPLES>();
    let mut audio = Audiograph::new(44100.0, Watcher::on(mixer));

    c.bench_function("mixer_parallelism", |b| {
        b.iter(|| audio.stream_into(&mut buf, true))
    });
    c.bench_function("mixer_sequential", |b| {
        b.iter(|| audio.stream_into(&mut buf, false))
    });
    c.bench_function("mixer_parallel_rtrb", |b| {
        b.iter(|| audio.stream_into_rtrb(true))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
