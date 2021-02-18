use alloc::boxed::Box;
use rume::*;

mod table {
    pub const SIZE: usize = 256;
    pub const FREQ: f32 = 48_000. / SIZE as f32;
    pub const TIME: f32 = 1. / FREQ;
}

#[processor]
pub struct Oscillator {
    #[input]
    frequency: f32,

    #[input]
    amplitude: f32,

    #[input]
    amount: f32,

    #[output]
    sample: f32,

    phase: [f32; 2],
    inv_sample_rate: f32,
    carrier: rume::OwnedLut,
    modulator: rume::OwnedLut,
}

impl Processor for Oscillator {
    fn prepare(&mut self, data: AudioConfig) {
        self.inv_sample_rate = 1.0 / data.sample_rate as f32;
        self.carrier = rume::OwnedLut::new(
            |x: f32| libm::sin(x as f64 * 2. * core::f64::consts::PI) as f32,
            table::SIZE,
        );
        self.modulator = self.carrier.clone();
    }

    fn process(&mut self) {
        const TWO_PI: f32 = 2.0_f32 * core::f32::consts::PI;

        let freq = self.frequency * table::TIME;

        self.modulator.phasor.inc(freq);
        self.sample = self.modulator.advance() * 0.5;

        self.carrier
            .phasor
            .inc(freq * self.amount * self.sample * 0.01);
        self.sample += self.carrier.advance() * 0.5;

        self.sample *= self.amplitude;
    }
}
rume::graph! {
    inputs: {
        freq: { init: 220.0, smooth: 4 },
        fm:   { init:   2.0, range:  0.1..16.0 },
        amp:  { init:   0.1, range:  0.0..0.8,   smooth: 10 },
    },
    outputs: {
        out,
    },
    processors: {
        osc: Oscillator::default(),
    },
    connections: {
        freq.output ->  osc.input.0,
        fm.output   ->  osc.input.1,
        amp.output  ->  osc.input.2,
        osc.output  ->  out.input,
    }
}
