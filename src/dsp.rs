use alloc::boxed::Box;
use rume::*;

mod table {
    pub const SIZE: usize = 256;
    pub const FREQ: f32 = 48_000. / SIZE as f32;
    pub const TIME: f32 = 1. / FREQ;
}

#[rume::processor]
pub struct Oscillator {
    #[input]
    frequency: f32,

    #[input]
    fm: f32,

    #[input]
    amplitude: f32,

    #[output]
    sample: f32,

    carrier: rume::OwnedLut,
    // modulator: rume::OwnedLut,
    sample_period: f32,
}

impl Oscillator {
    pub fn new() -> Self {
        let mut osc = Self::default();
        osc.carrier = rume::OwnedLut::new(
            |x: f32| libm::sin(x as f64 * 2. * core::f64::consts::PI) as f32,
            table::SIZE,
        );
        // osc.modulator = osc.carrier.clone();
        osc
    }
}

impl rume::Processor for Oscillator {
    fn prepare(&mut self, config: rume::AudioConfig) {
        self.sample_period = 1.0 / config.sample_rate as f32;
    }

    fn process(&mut self) {
        self.carrier.phasor.inc(self.frequency * table::TIME);
        self.sample = self.carrier.advance();

        /*
        self.carrier
            .phasor
            .inc(self.frequency * sample * self.fm * table::TIME);

        self.sample = (self.carrier.advance() + sample) * self.amplitude * 0.5;
        // self.sample = sample * 0.1;\
        */
    }
}

rume::graph! {
    inputs: {
        freq: { init: 220.0, range: 16.0..880.0, smooth: 10 },
        fm:   { init:   6.0, range:  0.1..16.0 },
        amp:  { init:   0.1, range:  0.0..0.8,   smooth: 10 },
    },
    outputs: {
        out,
    },
    processors: {
        osc: Oscillator::new(),
    },
    connections: {
        freq.output ->  osc.input.0,
        fm.output   ->  osc.input.1,
        amp.output  ->  osc.input.2,
        osc.output  ->  out.input,
    }
}
