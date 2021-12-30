use criterion::{criterion_group, criterion_main, Criterion};
use audio_graph::{Node, Watcher, Audiograph};
use audio_graph::node::{Process, Params};

struct SineWave {
    step: usize,
}

struct SineWaveParams {
    ampl: f32,
    freq: f32,
}

impl Params for SineWaveParams {}

impl SineWave {
    fn new() -> Self {
        let step = 0;
        Self {
            step,
        }
    }
}

impl Process<f32> for SineWave {
    type P = SineWaveParams;

    fn process_next_value(&mut self, params: &Self::P, _: &[f32]) -> f32 {
        self.step += 1;
        ((self.step as f32) / 44100.0 * params.freq).sin() * params.ampl
    }
}

/* Mixer */
struct Mixer;
impl Process<f32> for Mixer {
    type P = ();

    fn process_next_value(&mut self, _: &Self::P, inputs: &[f32]) -> f32 {
        inputs.iter().sum::<f32>()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let sw1 = Node::new(
        "sw1",
        SineWaveParams { ampl: 0.1, freq: 2500.0 },
        SineWave::new()
    );
    let sw2 = Node::new(
        "sw2",
        SineWaveParams { ampl: 0.02, freq: 9534.0 },
        SineWave::new()
    );
    let sw3 = Node::new(
        "sw3",
        SineWaveParams { ampl: 0.01, freq: 15534.0 },
        SineWave::new()
    );
    let mut mixer = Node::new(
        "mixer",
        (),
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