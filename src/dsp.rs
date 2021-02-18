#[macro_use]
use rume::*;

#[derive(Debug, Clone)]
pub struct Delay {
    pub input: (DelaySampleInput, DelayTimeInput),
    pub output: DelaySampleOutput,
    sample: f32,
    delay_ticks: f32,
    sample_rate: u32,
    memory: [f32; 44_100],
    read_idx: f32,
    write_idx: usize,
    buffer_size: usize,
}

impl Default for Delay {
    fn default() -> Delay {
        Delay {
            input: (DelaySampleInput, DelayTimeInput),
            output: DelaySampleOutput,
            sample: 0.0,
            delay_ticks: 0.0,
            sample_rate: 44_100,
            memory: [0.0; 44_100],
            read_idx: 0.0,
            write_idx: 0,
            buffer_size: 0,
        }
    }
}

input! { Delay, DelayTimeInput,
    |proc: &mut Delay, time_ms: f32| {
        proc.delay_ticks = (time_ms * 0.001) * proc.sample_rate as f32;
        proc.buffer_size = proc.memory.len();
    }
}

input! { Delay, DelaySampleInput,
    |proc: &mut Delay, sample: f32| {
        proc.sample = sample;
    }
}

output! { Delay, DelaySampleOutput,
    |proc: &mut Delay| -> f32 {
        proc.sample
    }
}

#[inline(always)]
fn lerp(a: f32, b: f32, w: f32) -> f32 {
    a + w * (b - a)
}

impl Processor for Delay {
    fn prepare(&mut self, data: AudioConfig) {
        self.sample_rate = data.sample_rate as u32;
    }

    fn process(&mut self) {
        let buffer_size = self.memory.len();

        self.memory[self.write_idx] = self.sample;
        self.write_idx = (self.write_idx + 1) % buffer_size;
        self.read_idx = (self.write_idx as f32 - self.delay_ticks) % buffer_size as f32;

        let read_idx_0 = self.read_idx as usize;
        let read_idx_1 = (read_idx_0 + 1) % buffer_size;

        let wet = lerp(
            self.memory[read_idx_0],
            self.memory[read_idx_1],
            self.read_idx % 1.0,
        );

        self.sample = lerp(self.sample, wet, 0.5);
    }
}

#[rume::processor]
pub struct Distortion {
    #[sample]
    sample: f32,

    #[input]
    amount: f32,
}

impl Processor for Distortion {
    fn prepare(&mut self, _: AudioConfig) {}

    #[inline(always)]
    fn process(&mut self) {
        self.sample = (self.amount * self.sample).tanh();
    }
}

mod table {
    pub const SIZE: usize = 256;
    pub const FREQ: f32 = 48_000. / SIZE as f32;
    pub const TIME: f32 = 1. / FREQ;
}

#[rume::processor]
pub struct Sine {
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

impl Processor for Sine {
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
        self.sample = self.modulator.advance();

        self.carrier.phasor.inc(freq * self.amount * self.sample);
        self.sample += self.carrier.advance();

        self.sample *= self.amplitude;
    }
}

#[derive(Debug, Clone)]
enum EnvelopeState {
    Off = 0,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvelopeState {
    fn default() -> EnvelopeState {
        EnvelopeState::Off
    }
}

#[rume::processor]
pub struct Envelope {
    #[output]
    amplitude: f32,

    sample_rate: f32,
    state: EnvelopeState,

    attack_delta: f32,

    decay_delta: f32,

    sustain_level: f32,

    release_delta: f32,

    #[input]
    note_on: f32,
    #[input]
    note_off: f32,
}

impl Processor for Envelope {
    fn prepare(&mut self, data: rume::AudioConfig) {
        self.sample_rate = data.sample_rate as f32;
        self.attack_delta = 0.01;
        self.decay_delta = 0.01;
        self.sustain_level = 0.1;
        self.release_delta = 0.01;
    }

    fn process(&mut self) {
        if self.note_on >= 1.0 {
            self.state = EnvelopeState::Attack;
            self.note_on = 0.0;
        }

        if self.note_off >= 1.0 {
            self.state = EnvelopeState::Release;
            self.note_off = 0.0;
        }

        match self.state {
            EnvelopeState::Attack => {
                self.amplitude += self.attack_delta;
                if self.amplitude >= 1.0 {
                    self.amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.amplitude -= self.decay_delta;
                if self.amplitude <= self.sustain_level {
                    if self.amplitude <= 0.0 {
                        self.state = EnvelopeState::Off;
                    } else {
                        self.amplitude = self.sustain_level;
                        self.state = EnvelopeState::Sustain;
                    }
                }
            }
            EnvelopeState::Sustain => {
                self.amplitude = self.sustain_level;
            }
            EnvelopeState::Release => {
                self.amplitude -= self.release_delta;
                if self.amplitude <= 0.0 {
                    self.amplitude = 0.0;
                    self.state = EnvelopeState::Off;
                }
            }
            EnvelopeState::Off => {
                self.amplitude = 0.0;
            }
        }
    }
}

rume::graph! {
    inputs: {
        freq: { init: 220.0, range: 16.0..880.0 },
        amp: { init: 0.05, range: 0.0..0.9 },
        fm_amt: { init: 1.0, range: 0.01..16.0 },

        note_on: { kind: trigger },
        note_off: { kind: trigger },
    },
    outputs: {
        audio_out,
    },
    processors: {
        sine: Sine::default(),
        env: Envelope::default(),
    },
    connections: {
        freq.output    ->  sine.input.0,
        env.output     ->  sine.input.1,
        fm_amt.output  ->  sine.input.2,
        sine.output    ->  audio_out.input,

        note_on.output  ->  env.input.0,
        note_off.output ->  env.input.1,
    }
}
