use alloc::boxed::Box;
use rume::*;

mod table {
    pub const SIZE: usize = 256;
    pub const FREQ: f32 = 44_100. / SIZE as f32;
    pub const TIME: f32 = 1. / FREQ;
}

#[processor]
pub struct Oscillator {
    #[input]
    freq: f32,

    #[input]
    amp_c: f32,

    #[input]
    amp_m: f32,

    #[input]
    ratio: f32,

    #[output]
    sample: f32,

    carrier: rume::OwnedLut,
    modulator: rume::OwnedLut,
}

impl Processor for Oscillator {
    fn prepare(&mut self, data: AudioConfig) {
        self.carrier = rume::OwnedLut::new(
            |x: f32| libm::sin(x as f64 * 2. * core::f64::consts::PI) as f32,
            table::SIZE,
        );
        self.modulator = self.carrier.clone();
    }

    fn process(&mut self) {
        let freq = self.freq * table::TIME;

        self.modulator.phasor.inc(freq * self.ratio);
        let mod_sample = self.modulator.advance() * self.amp_m;

        self.carrier.phasor.inc(freq * mod_sample);
        self.sample = self.carrier.advance() * self.amp_c;

        // self.sample = (car_sample + mod_sample) * 0.5;
    }
}
rume::graph! {
    inputs: {
        freq:   { init: 220.0,  smooth: 4 },
        amp_c:  { init: 1.0,    smooth: 4 },
        amp_m:  { init: 1.0,    smooth: 4 },
        ratio:  { init: 1.0,    smooth: 4 },
    },
    outputs: {
        out,
    },
    processors: {
        osc: Oscillator::default(),
    },
    connections: {
        freq.output     ->  osc.input.0,
        amp_c.output    ->  osc.input.1,
        amp_m.output    ->  osc.input.2,
        ratio.output    ->  osc.input.3,
        osc.output      ->  out.input,
    }
}
